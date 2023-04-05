use std::hash::BuildHasherDefault;

use crate::{
    ai::{casual, choose, finish, important, just, now, seq, until, Action, NpcCtx},
    data::{
        npc::{Brain, PathData},
        Sites,
    },
    event::OnTick,
    RtState, Rule, RuleError,
};
use common::{
    astar::{Astar, PathResult},
    path::Path,
    rtsim::{ChunkResource, Profession, SiteId},
    spiral::Spiral2d,
    store::Id,
    terrain::{SiteKindMeta, TerrainChunkSize},
    time::DayPeriod,
    vol::RectVolSize,
};
use fxhash::FxHasher64;
use itertools::{Either, Itertools};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use vek::*;
use world::{
    civ::{self, Track},
    site::{Site as WorldSite, SiteKind},
    site2::{self, PlotKind, TileKind},
    util::NEIGHBORS,
    IndexRef, World,
};

pub struct NpcAi;

const CARDINALS: &[Vec2<i32>] = &[
    Vec2::new(1, 0),
    Vec2::new(0, 1),
    Vec2::new(-1, 0),
    Vec2::new(0, -1),
];

fn path_in_site(start: Vec2<i32>, end: Vec2<i32>, site: &site2::Site) -> PathResult<Vec2<i32>> {
    let heuristic = |tile: &Vec2<i32>, _: &Vec2<i32>| tile.as_::<f32>().distance(end.as_());
    let mut astar = Astar::new(1000, start, BuildHasherDefault::<FxHasher64>::default());

    let transition = |a: &Vec2<i32>, b: &Vec2<i32>| {
        let distance = a.as_::<f32>().distance(b.as_());
        let a_tile = site.tiles.get(*a);
        let b_tile = site.tiles.get(*b);

        let terrain = match &b_tile.kind {
            TileKind::Empty => 3.0,
            TileKind::Hazard(_) => 50.0,
            TileKind::Field => 8.0,
            TileKind::Plaza | TileKind::Road { .. } | TileKind::Path => 1.0,

            TileKind::Building
            | TileKind::Castle
            | TileKind::Wall(_)
            | TileKind::Tower(_)
            | TileKind::Keep(_)
            | TileKind::Gate
            | TileKind::GnarlingFortification => 5.0,
        };
        let is_door_tile = |plot: Id<site2::Plot>, tile: Vec2<i32>| match site.plot(plot).kind() {
            site2::PlotKind::House(house) => house.door_tile == tile,
            site2::PlotKind::Workshop(_) => true,
            _ => false,
        };
        let building = if a_tile.is_building() && b_tile.is_road() {
            a_tile
                .plot
                .and_then(|plot| is_door_tile(plot, *a).then_some(1.0))
                .unwrap_or(10000.0)
        } else if b_tile.is_building() && a_tile.is_road() {
            b_tile
                .plot
                .and_then(|plot| is_door_tile(plot, *b).then_some(1.0))
                .unwrap_or(10000.0)
        } else if (a_tile.is_building() || b_tile.is_building()) && a_tile.plot != b_tile.plot {
            10000.0
        } else {
            1.0
        };

        distance * terrain + building
    };

    astar.poll(
        1000,
        heuristic,
        |&tile| CARDINALS.iter().map(move |c| tile + *c),
        transition,
        |tile| *tile == end || site.tiles.get_known(*tile).is_none(),
    )
}

fn path_between_sites(
    start: SiteId,
    end: SiteId,
    sites: &Sites,
    world: &World,
) -> PathResult<(Id<Track>, bool)> {
    let world_site = |site_id: SiteId| {
        let id = sites.get(site_id).and_then(|site| site.world_site)?;
        world.civs().sites.recreate_id(id.id())
    };

    let start = if let Some(start) = world_site(start) {
        start
    } else {
        return PathResult::Pending;
    };
    let end = if let Some(end) = world_site(end) {
        end
    } else {
        return PathResult::Pending;
    };

    let get_site = |site: &Id<civ::Site>| world.civs().sites.get(*site);

    let end_pos = get_site(&end).center.as_::<f32>();
    let heuristic =
        |site: &Id<civ::Site>, _: &Id<civ::Site>| get_site(site).center.as_().distance(end_pos);

    let mut astar = Astar::new(250, start, BuildHasherDefault::<FxHasher64>::default());

    let neighbors = |site: &Id<civ::Site>| world.civs().neighbors(*site);

    let track_between = |a: Id<civ::Site>, b: Id<civ::Site>| {
        world
            .civs()
            .tracks
            .get(world.civs().track_between(a, b).unwrap().0)
    };

    let transition = |a: &Id<civ::Site>, b: &Id<civ::Site>| track_between(*a, *b).cost;

    let path = astar.poll(250, heuristic, neighbors, transition, |site| *site == end);

    path.map(|path| {
        let path = path
            .into_iter()
            .tuple_windows::<(_, _)>()
            .map(|(a, b)| world.civs().track_between(a, b).unwrap())
            .collect_vec();
        Path { nodes: path }
    })
}

fn path_site(
    start: Vec2<f32>,
    end: Vec2<f32>,
    site: Id<WorldSite>,
    index: IndexRef,
) -> Option<Vec<Vec2<f32>>> {
    if let Some(site) = index.sites.get(site).site2() {
        let start = site.wpos_tile_pos(start.as_());

        let end = site.wpos_tile_pos(end.as_());

        let nodes = match path_in_site(start, end, site) {
            PathResult::Path(p) => p.nodes,
            PathResult::Exhausted(p) => p.nodes,
            PathResult::None(_) | PathResult::Pending => return None,
        };

        Some(
            nodes
                .into_iter()
                .map(|tile| site.tile_center_wpos(tile).as_() + 0.5)
                .collect(),
        )
    } else {
        None
    }
}

fn path_towns(
    start: SiteId,
    end: SiteId,
    sites: &Sites,
    world: &World,
) -> Option<PathData<(Id<Track>, bool), SiteId>> {
    match path_between_sites(start, end, sites, world) {
        PathResult::Exhausted(p) => Some(PathData {
            end,
            path: p.nodes.into(),
            repoll: true,
        }),
        PathResult::Path(p) => Some(PathData {
            end,
            path: p.nodes.into(),
            repoll: false,
        }),
        PathResult::Pending | PathResult::None(_) => None,
    }
}

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|ctx| {
            // Temporarily take the brains of NPCs out of their heads to appease the borrow
            // checker
            let mut npc_data = {
                let mut data = ctx.state.data_mut();
                data.npcs
                    .iter_mut()
                    .map(|(npc_id, npc)| {
                        let controller = std::mem::take(&mut npc.controller);
                        let brain = npc.brain.take().unwrap_or_else(|| Brain {
                            action: Box::new(think().repeat()),
                        });
                        (npc_id, controller, brain)
                    })
                    .collect::<Vec<_>>()
            };

            // Do a little thinking
            {
                let data = &*ctx.state.data();

                npc_data
                    .par_iter_mut()
                    .for_each(|(npc_id, controller, brain)| {
                        let npc = &data.npcs[*npc_id];

                        brain.action.tick(&mut NpcCtx {
                            state: ctx.state,
                            world: ctx.world,
                            index: ctx.index,
                            time_of_day: ctx.event.time_of_day,
                            time: ctx.event.time,
                            npc,
                            npc_id: *npc_id,
                            controller,
                            rng: ChaChaRng::from_seed(thread_rng().gen::<[u8; 32]>()),
                        });
                    });
            }

            // Reinsert NPC brains
            let mut data = ctx.state.data_mut();
            for (npc_id, controller, brain) in npc_data {
                data.npcs[npc_id].controller = controller;
                data.npcs[npc_id].brain = Some(brain);
            }
        });

        Ok(Self)
    }
}

fn idle() -> impl Action { just(|ctx| ctx.controller.do_idle()).debug(|| "idle") }

/// Try to walk toward a 3D position without caring for obstacles.
fn goto(wpos: Vec3<f32>, speed_factor: f32, goal_dist: f32) -> impl Action {
    const STEP_DIST: f32 = 24.0;
    const WAYPOINT_DIST: f32 = 12.0;

    let mut waypoint = None;

    just(move |ctx| {
        let rpos = wpos - ctx.npc.wpos;
        let len = rpos.magnitude();

        // If we're close to the next waypoint, complete it
        if waypoint.map_or(false, |waypoint: Vec3<f32>| {
            ctx.npc.wpos.xy().distance_squared(waypoint.xy()) < WAYPOINT_DIST.powi(2)
        }) {
            waypoint = None;
        }

        // Get the next waypoint on the route toward the goal
        let waypoint = waypoint.get_or_insert_with(|| {
            let wpos = ctx.npc.wpos + (rpos / len) * len.min(STEP_DIST);

            wpos.with_z(
                ctx.world
                    .sim()
                    .get_surface_alt_approx(wpos.xy().as_())
                    .unwrap_or(wpos.z),
            )
        });

        ctx.controller.do_goto(*waypoint, speed_factor);
    })
    .repeat()
    .stop_if(move |ctx| ctx.npc.wpos.xy().distance_squared(wpos.xy()) < goal_dist.powi(2))
    .debug(move || format!("goto {}, {}, {}", wpos.x, wpos.y, wpos.z))
    .map(|_| {})
}

/// Try to walk toward a 2D position on the terrain without caring for
/// obstacles.
fn goto_2d(wpos2d: Vec2<f32>, speed_factor: f32, goal_dist: f32) -> impl Action {
    now(move |ctx| {
        let wpos = wpos2d.with_z(ctx.world.sim().get_alt_approx(wpos2d.as_()).unwrap_or(0.0));
        goto(wpos, speed_factor, goal_dist)
    })
}

fn traverse_points<F>(mut next_point: F, speed_factor: f32) -> impl Action
where
    F: FnMut(&mut NpcCtx) -> Option<Vec2<f32>> + Send + Sync + 'static,
{
    until(move |ctx| {
        let wpos = next_point(ctx)?;

        let wpos_site = |wpos: Vec2<f32>| {
            ctx.world
                .sim()
                .get(wpos.as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_())
                .and_then(|chunk| chunk.sites.first().copied())
        };

        // If we're traversing within a site, to intra-site pathfinding
        if let Some(site) = wpos_site(wpos) {
            let mut site_exit = wpos;
            while let Some(next) = next_point(ctx).filter(|next| wpos_site(*next) == Some(site)) {
                site_exit = next;
            }

            if let Some(path) = path_site(wpos, site_exit, site, ctx.index) {
                Some(Either::Left(
                    seq(path.into_iter().map(|wpos| goto_2d(wpos, 1.0, 8.0))).then(goto_2d(
                        site_exit,
                        speed_factor,
                        8.0,
                    )),
                ))
            } else {
                Some(Either::Right(goto_2d(site_exit, speed_factor, 8.0)))
            }
        } else {
            Some(Either::Right(goto_2d(wpos, speed_factor, 8.0)))
        }
    })
}

/// Try to travel to a site. Where practical, paths will be taken.
fn travel_to_point(wpos: Vec2<f32>, speed_factor: f32) -> impl Action {
    now(move |ctx| {
        const WAYPOINT: f32 = 48.0;
        let start = ctx.npc.wpos.xy();
        let diff = wpos - start;
        let n = (diff.magnitude() / WAYPOINT).max(1.0);
        let mut points = (1..n as usize + 1).map(move |i| start + diff * (i as f32 / n));
        traverse_points(move |_| points.next(), speed_factor)
    })
    .debug(|| "travel to point")
}

/// Try to travel to a site. Where practical, paths will be taken.
fn travel_to_site(tgt_site: SiteId, speed_factor: f32) -> impl Action {
    now(move |ctx| {
        let sites = &ctx.state.data().sites;

        // If we're currently in a site, try to find a path to the target site via
        // tracks
        if let Some(current_site) = ctx.npc.current_site
            && let Some(tracks) = path_towns(current_site, tgt_site, sites, ctx.world)
        {

            let mut nodes = tracks.path
                .into_iter()
                .flat_map(move |(track_id, reversed)| (0..)
                    .map(move |node_idx| (node_idx, track_id, reversed)));

            traverse_points(move |ctx| {
                let (node_idx, track_id, reversed) = nodes.next()?;
                let nodes = &ctx.world.civs().tracks.get(track_id).path().nodes;

                // Handle the case where we walk paths backward
                let idx = if reversed {
                    nodes.len().checked_sub(node_idx + 1)
                } else {
                    Some(node_idx)
                };

                if let Some(node) = idx.and_then(|idx| nodes.get(idx)) {
                    // Find the centre of the track node's chunk
                    let node_chunk_wpos = TerrainChunkSize::center_wpos(*node);

                    // Refine the node position a bit more based on local path information
                    Some(ctx.world.sim()
                        .get_nearest_path(node_chunk_wpos)
                        .map_or(node_chunk_wpos, |(_, wpos, _, _)| wpos.as_())
                        .as_::<f32>())
                } else {
                    None
                }
            }, speed_factor)
                .boxed()

            // For every track in the path we discovered between the sites...
            // seq(tracks
            //     .path
            //     .into_iter()
            //     .enumerate()
            //     // ...traverse the nodes of that path.
            //     .map(move |(i, (track_id, reversed))| now(move |ctx| {
            //         let track_len = ctx.world.civs().tracks.get(track_id).path().len();
            //         // Tracks can be traversed backward (i.e: from end to beginning). Account for this.
            //         seq(if reversed {
            //             Either::Left((0..track_len).rev())
            //         } else {
            //             Either::Right(0..track_len)
            //         }
            //             .enumerate()
            //             .map(move |(i, node_idx)| now(move |ctx| {
            //                 // Find the centre of the track node's chunk
            //                 let node_chunk_wpos = TerrainChunkSize::center_wpos(ctx.world
            //                     .civs()
            //                     .tracks
            //                     .get(track_id)
            //                     .path()
            //                     .nodes[node_idx]);

            //                 // Refine the node position a bit more based on local path information
            //                 let node_wpos = ctx.world.sim()
            //                     .get_nearest_path(node_chunk_wpos)
            //                     .map_or(node_chunk_wpos, |(_, wpos, _, _)| wpos.as_());

            //                 // Walk toward the node
            //                 goto_2d(node_wpos.as_(), 1.0, 8.0)
            //                     .debug(move || format!("traversing track node ({}/{})", i + 1, track_len))
            //             })))
            //     })
            //         .debug(move || format!("travel via track {:?} ({}/{})", track_id, i + 1, track_count))))
            //     .boxed()
        } else if let Some(site) = sites.get(tgt_site) {
            // If all else fails, just walk toward the target site in a straight line
            travel_to_point(site.wpos.map(|e| e as f32 + 0.5), speed_factor).boxed()
        } else {
            // If we can't find a way to get to the site at all, there's nothing more to be done
            finish().boxed()
        }
    })
    .debug(move || format!("travel_to_site {:?}", tgt_site))
}

// Seconds
fn timeout(time: f64) -> impl FnMut(&mut NpcCtx) -> bool + Clone + Send + Sync {
    let mut timeout = None;
    move |ctx| ctx.time.0 > *timeout.get_or_insert(ctx.time.0 + time)
}

fn socialize() -> impl Action {
    now(|ctx| {
        // TODO: Bit odd, should wait for a while after greeting
        if ctx.rng.gen_bool(0.002) && let Some(other) = ctx
            .state
            .data()
            .npcs
            .nearby(Some(ctx.npc_id), ctx.npc.wpos.xy(), 8.0)
            .choose(&mut ctx.rng)
        {
            just(move |ctx| ctx.controller.greet(other)).boxed()
        } else if ctx.rng.gen_bool(0.0003) {
            just(|ctx| ctx.controller.do_dance())
                .repeat()
                .stop_if(timeout(6.0))
                .debug(|| "dancing")
                .map(|_| ())
                .boxed()
        } else {
            idle().boxed()
        }
    })
}

fn adventure() -> impl Action {
    choose(|ctx| {
        // Choose a random site that's fairly close by
        if let Some(tgt_site) = ctx
            .state
            .data()
            .sites
            .iter()
            .filter(|(site_id, site)| {
                // Only path toward towns
                matches!(
                    site.world_site.map(|ws| &ctx.index.sites.get(ws).kind),
                    Some(
                        SiteKind::Refactor(_)
                            | SiteKind::CliffTown(_)
                            | SiteKind::SavannahPit(_)
                            | SiteKind::DesertCity(_)
                    ),
                ) && ctx.npc.current_site.map_or(true, |cs| *site_id != cs)
                    && ctx.rng.gen_bool(0.25)
            })
            .min_by_key(|(_, site)| site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32)
            .map(|(site_id, _)| site_id)
        {
            let wait_time = if matches!(ctx.npc.profession, Some(Profession::Merchant)) {
                60.0 * 15.0
            } else {
                60.0 * 3.0
            };
            let site_name = ctx.state.data().sites[tgt_site].world_site
                .map(|ws| ctx.index.sites.get(ws).name().to_string())
                .unwrap_or_default();
            // Travel to the site
            important(just(move |ctx| ctx.controller.say(format!("I've spent enough time here, onward to {}!", site_name)))
                .then(travel_to_site(tgt_site, 0.6))
                // Stop for a few minutes
                .then(villager(tgt_site).repeat().stop_if(timeout(wait_time)))
                .map(|_| ())
                .boxed(),
            )
        } else {
            casual(finish().boxed())
        }
    })
    .debug(move || "adventure")
}

fn gather_ingredients() -> impl Action {
    just(|ctx| {
        ctx.controller.do_gather(
            &[
                ChunkResource::Fruit,
                ChunkResource::Mushroom,
                ChunkResource::Plant,
            ][..],
        )
    })
    .debug(|| "gather ingredients")
}

fn hunt_animals() -> impl Action {
    just(|ctx| ctx.controller.do_hunt_animals()).debug(|| "hunt_animals")
}

fn find_forest(ctx: &mut NpcCtx) -> Option<Vec2<f32>> {
    let chunk_pos = ctx.npc.wpos.xy().as_() / TerrainChunkSize::RECT_SIZE.as_();
    Spiral2d::new()
        .skip(ctx.rng.gen_range(1..=8))
        .take(49)
        .map(|rpos| chunk_pos + rpos)
        .find(|cpos| {
            ctx.world
                .sim()
                .get(*cpos)
                .map_or(false, |c| c.tree_density > 0.75 && c.surface_veg > 0.5)
        })
        .map(|chunk| TerrainChunkSize::center_wpos(chunk).as_())
}

fn villager(visiting_site: SiteId) -> impl Action {
    choose(move |ctx| {
        /*
        if ctx
            .state
            .data()
            .sites
            .get(visiting_site)
            .map_or(true, |s| s.world_site.is_none())
        {
            return casual(idle()
                .debug(|| "idling (visiting site does not exist, perhaps it's stale data?)"));
        } else if ctx.npc.current_site != Some(visiting_site) {
            let npc_home = ctx.npc.home;
            // Travel to the site we're supposed to be in
            return urgent(travel_to_site(visiting_site, 1.0).debug(move || {
                if npc_home == Some(visiting_site) {
                    "travel home".to_string()
                } else {
                    "travel to visiting site".to_string()
                }
            }));
        } else
        */
        if DayPeriod::from(ctx.time_of_day.0).is_dark()
            && !matches!(ctx.npc.profession, Some(Profession::Guard))
        {
            return important(
                now(move |ctx| {
                    if let Some(house_wpos) = ctx
                        .state
                        .data()
                        .sites
                        .get(visiting_site)
                        .and_then(|site| ctx.index.sites.get(site.world_site?).site2())
                        .and_then(|site2| {
                            // Find a house in the site we're visiting
                            let house = site2
                                .plots()
                                .filter(|p| matches!(p.kind(), PlotKind::House(_)))
                                .choose(&mut ctx.rng)?;
                            Some(site2.tile_center_wpos(house.root_tile()).as_())
                        })
                    {
                        just(|ctx| ctx.controller.say("It's dark, time to go home"))
                            .then(travel_to_point(house_wpos, 0.65))
                            .debug(|| "walk to house")
                            .then(socialize().repeat().debug(|| "wait in house"))
                            .stop_if(|ctx| DayPeriod::from(ctx.time_of_day.0).is_light())
                            .then(just(|ctx| ctx.controller.say("A new day begins!")))
                            .map(|_| ())
                            .boxed()
                    } else {
                        finish().boxed()
                    }
                })
                .debug(|| "find somewhere to sleep"),
            );
        // Villagers with roles should perform those roles
        } else if matches!(ctx.npc.profession, Some(Profession::Herbalist)) && ctx.rng.gen_bool(0.8)
        {
            if let Some(forest_wpos) = find_forest(ctx) {
                return casual(
                    travel_to_point(forest_wpos, 0.5)
                        .debug(|| "walk to forest")
                        .then({
                            let wait_time = ctx.rng.gen_range(10.0..30.0);
                            gather_ingredients().repeat().stop_if(timeout(wait_time))
                        })
                        .map(|_| ()),
                );
            }
        } else if matches!(ctx.npc.profession, Some(Profession::Hunter)) && ctx.rng.gen_bool(0.8) {
            if let Some(forest_wpos) = find_forest(ctx) {
                return casual(
                    just(|ctx| ctx.controller.say("Time to go hunting!"))
                        .then(travel_to_point(forest_wpos, 0.75))
                        .debug(|| "walk to forest")
                        .then({
                            let wait_time = ctx.rng.gen_range(30.0..60.0);
                            hunt_animals().repeat().stop_if(timeout(wait_time))
                        })
                        .map(|_| ()),
                );
            }
        } else if matches!(ctx.npc.profession, Some(Profession::Merchant)) && ctx.rng.gen_bool(0.8)
        {
            return casual(
                just(|ctx| {
                    ctx.controller.say(
                        *[
                            "All my goods are of the highest quality!",
                            "Does anybody want to buy my wares?",
                            "I've got the best offers in town.",
                            "Looking for supplies? I've got you covered.",
                        ]
                        .iter()
                        .choose(&mut ctx.rng)
                        .unwrap(),
                    ) // Can't fail
                })
                .then(idle().repeat().stop_if(timeout(8.0)))
                .repeat()
                .stop_if(timeout(60.0))
                .debug(|| "sell wares")
                .map(|_| ()),
            );
        }

        // If nothing else needs doing, walk between plazas and socialize
        casual(now(move |ctx| {
            // Choose a plaza in the site we're visiting to walk to
            if let Some(plaza_wpos) = ctx
                .state
                .data()
                .sites
                .get(visiting_site)
                .and_then(|site| ctx.index.sites.get(site.world_site?).site2())
                .and_then(|site2| {
                    let plaza = &site2.plots[site2.plazas().choose(&mut ctx.rng)?];
                    Some(site2.tile_center_wpos(plaza.root_tile()).as_())
                })
            {
                // Walk to the plaza...
                Either::Left(travel_to_point(plaza_wpos, 0.5)
                    .debug(|| "walk to plaza"))
            } else {
                // No plazas? :(
                Either::Right(finish())
            }
                // ...then socialize for some time before moving on
                .then(socialize()
                    .repeat()
                    .stop_if(timeout(ctx.rng.gen_range(30.0..90.0)))
                    .debug(|| "wait at plaza"))
                .map(|_| ())
        }))
    })
    .debug(move || format!("villager at site {:?}", visiting_site))
}

/*
fn follow(npc: NpcId, distance: f32) -> impl Action {
    const STEP_DIST: f32 = 1.0;
    now(move |ctx| {
        if let Some(npc) = ctx.state.data().npcs.get(npc) {
            let d = npc.wpos.xy() - ctx.npc.wpos.xy();
            let len = d.magnitude();
            let dir = d / len;
            let wpos = ctx.npc.wpos.xy() + dir * STEP_DIST.min(len - distance);
            goto_2d(wpos, 1.0, distance).boxed()
        } else {
            // The npc we're trying to follow doesn't exist.
            finish().boxed()
        }
    })
    .repeat()
    .debug(move || format!("Following npc({npc:?})"))
    .map(|_| {})
}
*/

fn chunk_path(
    from: Vec2<i32>,
    to: Vec2<i32>,
    chunk_height: impl Fn(Vec2<i32>) -> Option<i32>,
) -> Box<dyn Action> {
    let heuristics =
        |(p, _): &(Vec2<i32>, i32), _: &(Vec2<i32>, i32)| p.distance_squared(to) as f32;
    let start = (from, chunk_height(from).unwrap());
    let mut astar = Astar::new(1000, start, BuildHasherDefault::<FxHasher64>::default());

    let path = astar.poll(
        1000,
        heuristics,
        |&(p, _)| {
            NEIGHBORS
                .into_iter()
                .map(move |n| p + n)
                .filter_map(|p| Some((p, chunk_height(p)?)))
        },
        |(p0, h0), (p1, h1)| {
            let diff =
                ((p0 - p1).as_() * TerrainChunkSize::RECT_SIZE.as_()).with_z((h0 - h1) as f32);

            diff.magnitude_squared()
        },
        |(e, _)| *e == to,
    );
    let path = match path {
        PathResult::Exhausted(p) | PathResult::Path(p) => p,
        _ => return finish().boxed(),
    };
    let len = path.len();
    seq(path
        .into_iter()
        .enumerate()
        .map(move |(i, (chunk_pos, height))| {
            let wpos = TerrainChunkSize::center_wpos(chunk_pos)
                .with_z(height)
                .as_();
            goto(wpos, 1.0, 5.0)
                .debug(move || format!("chunk path {i}/{len} chunk: {chunk_pos}, height: {height}"))
        }))
    .boxed()
}

fn pilot() -> impl Action {
    // Travel between different towns in a straight line
    now(|ctx| {
        let data = &*ctx.state.data();
        let site = data
            .sites
            .iter()
            .filter(|(id, _)| Some(*id) != ctx.npc.current_site)
            .filter(|(_, site)| {
                site.world_site
                    .and_then(|site| ctx.index.sites.get(site).kind.convert_to_meta())
                    .map_or(false, |meta| matches!(meta, SiteKindMeta::Settlement(_)))
            })
            .choose(&mut ctx.rng);
        if let Some((_id, site)) = site {
            let start_chunk =
                ctx.npc.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_::<i32>();
            let end_chunk = site.wpos / TerrainChunkSize::RECT_SIZE.as_::<i32>();
            chunk_path(start_chunk, end_chunk, |chunk| {
                ctx.world
                    .sim()
                    .get_alt_approx(TerrainChunkSize::center_wpos(chunk))
                    .map(|f| (f + 150.0) as i32)
            })
        } else {
            finish().boxed()
        }
    })
    .repeat()
    .map(|_| ())
}

fn captain() -> impl Action {
    // For now just randomly travel the sea
    now(|ctx| {
        let chunk = ctx.npc.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_::<i32>();
        if let Some(chunk) = NEIGHBORS
            .into_iter()
            .map(|neighbor| chunk + neighbor)
            .filter(|neighbor| {
                ctx.world
                    .sim()
                    .get(*neighbor)
                    .map_or(false, |c| c.river.river_kind.is_some())
            })
            .choose(&mut ctx.rng)
        {
            let wpos = TerrainChunkSize::center_wpos(chunk);
            let wpos = wpos.as_().with_z(
                ctx.world
                    .sim()
                    .get_interpolated(wpos, |chunk| chunk.water_alt)
                    .unwrap_or(0.0),
            );
            goto(wpos, 0.7, 5.0).boxed()
        } else {
            idle().boxed()
        }
    })
    .repeat()
    .map(|_| ())
}

fn humanoid() -> impl Action {
    choose(|ctx| {
        if let Some(riding) = &ctx.npc.riding {
            if riding.steering {
                if let Some(vehicle) = ctx.state.data().npcs.vehicles.get(riding.vehicle) {
                    match vehicle.body {
                        common::comp::ship::Body::DefaultAirship
                        | common::comp::ship::Body::AirBalloon => important(pilot()),
                        common::comp::ship::Body::SailBoat | common::comp::ship::Body::Galleon => {
                            important(captain())
                        },
                        _ => casual(idle()),
                    }
                } else {
                    casual(finish())
                }
            } else {
                important(socialize())
            }
        } else if matches!(
            ctx.npc.profession,
            Some(Profession::Adventurer(_) | Profession::Merchant)
        ) {
            casual(adventure())
        } else if let Some(home) = ctx.npc.home {
            casual(villager(home))
        } else {
            casual(finish()) // Homeless
        }
    })
}

fn bird_large() -> impl Action {
    choose(|ctx| {
        let data = ctx.state.data();
        if let Some(home) = ctx.npc.home {
            let is_home = ctx.npc.current_site.map_or(false, |site| home == site);
            if is_home {
                if let Some((_, site)) = data
                    .sites
                    .iter()
                    .filter(|(id, site)| {
                        *id != home
                            && site.world_site.map_or(false, |site| {
                                matches!(ctx.index.sites.get(site).kind, SiteKind::Dungeon(_))
                            })
                    })
                    .choose(&mut ctx.rng)
                {
                    casual(goto(
                        site.wpos.as_::<f32>().with_z(
                            ctx.world
                                .sim()
                                .get_surface_alt_approx(site.wpos)
                                .unwrap_or(0.0)
                                + ctx.npc.body.flying_height(),
                        ),
                        1.0,
                        20.0,
                    ))
                } else {
                    casual(idle())
                }
            } else if let Some(site) = data.sites.get(home) {
                casual(goto(
                    site.wpos.as_::<f32>().with_z(
                        ctx.world
                            .sim()
                            .get_surface_alt_approx(site.wpos)
                            .unwrap_or(0.0)
                            + ctx.npc.body.flying_height(),
                    ),
                    1.0,
                    20.0,
                ))
            } else {
                casual(idle())
            }
        } else {
            casual(idle())
        }
    })
}

fn think() -> impl Action {
    choose(|ctx| match ctx.npc.body {
        common::comp::Body::Humanoid(_) => casual(humanoid()),
        common::comp::Body::BirdLarge(_) => casual(bird_large()),
        _ => casual(socialize()),
    })
}
