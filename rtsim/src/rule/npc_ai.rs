use std::{collections::VecDeque, hash::BuildHasherDefault};

use crate::{
    ai::{casual, choose, finish, important, just, now, seq, urgent, watch, Action, NpcCtx},
    data::{
        npc::{Brain, Controller, Npc, NpcId, PathData, PathingMemory},
        Sites,
    },
    event::OnTick,
    EventCtx, RtState, Rule, RuleError,
};
use common::{
    astar::{Astar, PathResult},
    path::Path,
    rtsim::{Profession, SiteId},
    store::Id,
    terrain::TerrainChunkSize,
    time::DayPeriod,
    vol::RectVolSize,
};
use fxhash::FxHasher64;
use itertools::Itertools;
use rand::prelude::*;
use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    ops::ControlFlow,
};
use vek::*;
use world::{
    civ::{self, Track},
    site::{Site as WorldSite, SiteKind},
    site2::{self, PlotKind, TileKind},
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
    let heuristic = |tile: &Vec2<i32>| tile.as_::<f32>().distance(end.as_());
    let mut astar = Astar::new(
        1000,
        start,
        &heuristic,
        BuildHasherDefault::<FxHasher64>::default(),
    );

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
                .and_then(|plot| is_door_tile(plot, *a).then(|| 1.0))
                .unwrap_or(10000.0)
        } else if b_tile.is_building() && a_tile.is_road() {
            b_tile
                .plot
                .and_then(|plot| is_door_tile(plot, *b).then(|| 1.0))
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
    let heuristic = |site: &Id<civ::Site>| get_site(site).center.as_().distance(end_pos);

    let mut astar = Astar::new(
        250,
        start,
        heuristic,
        BuildHasherDefault::<FxHasher64>::default(),
    );

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

fn path_town(
    wpos: Vec3<f32>,
    site: Id<WorldSite>,
    index: IndexRef,
    end: impl FnOnce(&site2::Site) -> Option<Vec2<i32>>,
) -> Option<PathData<Vec2<i32>, Vec2<i32>>> {
    match &index.sites.get(site).kind {
        SiteKind::Refactor(site) | SiteKind::CliffTown(site) | SiteKind::DesertCity(site) => {
            let start = site.wpos_tile_pos(wpos.xy().as_());

            let end = end(site)?;

            if start == end {
                return None;
            }

            // We pop the first element of the path
            // fn pop_first<T>(mut queue: VecDeque<T>) -> VecDeque<T> {
            //     queue.pop_front();
            //     queue
            // }

            match path_in_site(start, end, site) {
                PathResult::Path(p) => Some(PathData {
                    end,
                    path: p.nodes.into(), //pop_first(p.nodes.into()),
                    repoll: false,
                }),
                PathResult::Exhausted(p) => Some(PathData {
                    end,
                    path: p.nodes.into(), //pop_first(p.nodes.into()),
                    repoll: true,
                }),
                PathResult::None(_) | PathResult::Pending => None,
            }
        },
        _ => {
            // No brain T_T
            None
        },
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

const MAX_STEP: f32 = 32.0;

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|mut ctx| {
            let npc_ids = ctx.state.data().npcs.keys().collect::<Vec<_>>();

            for npc_id in npc_ids {
                let mut brain = ctx.state.data_mut().npcs[npc_id]
                    .brain
                    .take()
                    .unwrap_or_else(|| Brain {
                        action: Box::new(think().repeat()),
                    });

                let controller = {
                    let data = &*ctx.state.data();
                    let npc = &data.npcs[npc_id];

                    let mut controller = Controller { goto: npc.goto };

                    brain.action.tick(&mut NpcCtx {
                        state: ctx.state,
                        world: ctx.world,
                        index: ctx.index,
                        time_of_day: ctx.event.time_of_day,
                        time: ctx.event.time,
                        npc,
                        npc_id,
                        controller: &mut controller,
                    });

                    /*
                    let action: ControlFlow<()> = try {
                        brain.tick(&mut NpcData {
                            ctx: &ctx,
                            npc,
                            npc_id,
                            controller: &mut controller,
                        });
                        /*
                        // // Choose a random plaza in the npcs home site (which should be the
                        // // current here) to go to.
                        let task =
                            generate(move |(_, npc, ctx): &(NpcId, &Npc, &EventCtx<_, _>)| {
                                let data = ctx.state.data();
                                let site2 =
                                    npc.home.and_then(|home| data.sites.get(home)).and_then(
                                        |home| match &ctx.index.sites.get(home.world_site?).kind
                                        {
                                            SiteKind::Refactor(site2)
                                            | SiteKind::CliffTown(site2)
                                            | SiteKind::DesertCity(site2) => Some(site2),
                                            _ => None,
                                        },
                                    );

                                let wpos = site2
                                    .and_then(|site2| {
                                        let plaza = &site2.plots
                                            [site2.plazas().choose(&mut thread_rng())?];
                                        Some(site2.tile_center_wpos(plaza.root_tile()).as_())
                                    })
                                    .unwrap_or(npc.wpos.xy());

                                TravelTo {
                                    wpos,
                                    use_paths: true,
                                }
                            })
                            .repeat();

                        task_state.perform(task, &(npc_id, &*npc, &ctx), &mut controller)?;
                        */
                    };
                    */

                    controller
                };

                ctx.state.data_mut().npcs[npc_id].goto = controller.goto;
                ctx.state.data_mut().npcs[npc_id].brain = Some(brain);
            }
        });

        Ok(Self)
    }
}

fn idle() -> impl Action { just(|ctx| *ctx.controller = Controller::idle()).debug(|| "idle") }

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
        let waypoint =
            waypoint.get_or_insert_with(|| ctx.npc.wpos + (rpos / len) * len.min(STEP_DIST));

        ctx.controller.goto = Some((*waypoint, speed_factor));
    })
    .repeat()
    .stop_if(move |ctx| ctx.npc.wpos.xy().distance_squared(wpos.xy()) < goal_dist.powi(2))
    .debug(move || format!("goto {}, {}, {}", wpos.x, wpos.y, wpos.z))
    .map(|_| {})
}

/// Try to walk toward a 2D position on the terrain without caring for
/// obstacles.
fn goto_2d(wpos2d: Vec2<f32>, speed_factor: f32, goal_dist: f32) -> impl Action {
    const MIN_DIST: f32 = 2.0;

    now(move |ctx| {
        let wpos = wpos2d.with_z(ctx.world.sim().get_alt_approx(wpos2d.as_()).unwrap_or(0.0));
        goto(wpos, speed_factor, goal_dist)
    })
}

/// Try to travel to a site. Where practical, paths will be taken.
fn travel_to_site(tgt_site: SiteId) -> impl Action {
    now(move |ctx| {
        let sites = &ctx.state.data().sites;

        // If we're currently in a site, try to find a path to the target site via
        // tracks
        if let Some(current_site) = ctx.npc.current_site
            && let Some(tracks) = path_towns(current_site, tgt_site, sites, ctx.world)
        {
            let track_count = tracks.path.len();
            // For every track in the path we discovered between the sites...
            seq(tracks
                .path
                .into_iter()
                .enumerate()
                // ...traverse the nodes of that path.
                .map(move |(i, (track_id, reversed))| now(move |ctx| {
                    let track_len = ctx.world.civs().tracks.get(track_id).path().len();
                    // Tracks can be traversed backward (i.e: from end to beginning). Account for this.
                    seq(if reversed {
                        itertools::Either::Left((0..track_len).rev())
                    } else {
                        itertools::Either::Right(0..track_len)
                    }
                        .enumerate()
                        .map(move |(i, node_idx)| now(move |ctx| {
                            // Find the centre of the track node's chunk
                            let node_chunk_wpos = TerrainChunkSize::center_wpos(ctx.world
                                .civs()
                                .tracks
                                .get(track_id)
                                .path()
                                .nodes[node_idx]);

                            // Refine the node position a bit more based on local path information
                            let node_wpos = ctx.world.sim()
                                .get_nearest_path(node_chunk_wpos)
                                .map_or(node_chunk_wpos, |(_, wpos, _, _)| wpos.as_());

                            // Walk toward the node
                            goto_2d(node_wpos.as_(), 1.0, 8.0)
                                .debug(move || format!("traversing track node ({}/{})", i + 1, track_len))
                        })))
                })
                    .debug(move || format!("travel via track {:?} ({}/{})", track_id, i + 1, track_count))))
                .boxed()
        } else if let Some(site) = sites.get(tgt_site) {
            // If all else fails, just walk toward the target site in a straight line
            goto_2d(site.wpos.map(|e| e as f32 + 0.5), 1.0, 8.0).boxed()
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
                    && thread_rng().gen_bool(0.25)
            })
            .min_by_key(|(_, site)| site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32)
            .map(|(site_id, _)| site_id)
        {
            // Travel to the site
            important(
                travel_to_site(tgt_site)
                // Stop for a few minutes
                .then(villager(tgt_site).repeat().stop_if(timeout(60.0 * 3.0)))
                .map(|_| ())
                .boxed(),
            )
        } else {
            casual(finish().boxed())
        }
    })
    .debug(move || format!("adventure"))
}

fn villager(visiting_site: SiteId) -> impl Action {
    choose(move |ctx| {
        if ctx.npc.current_site != Some(visiting_site) {
            let npc_home = ctx.npc.home;
            // Travel to the site we're supposed to be in
            urgent(travel_to_site(visiting_site).debug(move || {
                if npc_home == Some(visiting_site) {
                    format!("travel home")
                } else {
                    format!("travel to visiting site")
                }
            }))
        } else if DayPeriod::from(ctx.time_of_day.0).is_dark()
            && !matches!(ctx.npc.profession, Some(Profession::Guard))
        {
            important(
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
                                .choose(&mut thread_rng())?;
                            Some(site2.tile_center_wpos(house.root_tile()).as_())
                        })
                    {
                        goto_2d(house_wpos, 0.5, 1.0)
                            .debug(|| "walk to house")
                            .then(idle().repeat().debug(|| "wait in house"))
                            .stop_if(|ctx| DayPeriod::from(ctx.time_of_day.0).is_light())
                            .map(|_| ())
                            .boxed()
                    } else {
                        finish().boxed()
                    }
                })
                .debug(|| "find somewhere to sleep"),
            )
        } else {
            casual(now(move |ctx| {
                // Choose a plaza in the site we're visiting to walk to
                if let Some(plaza_wpos) = ctx
                    .state
                    .data()
                    .sites
                    .get(visiting_site)
                    .and_then(|site| ctx.index.sites.get(site.world_site?).site2())
                    .and_then(|site2| {
                        let plaza = &site2.plots[site2.plazas().choose(&mut thread_rng())?];
                        Some(site2.tile_center_wpos(plaza.root_tile()).as_())
                    })
                {
                    // Walk to the plaza...
                    goto_2d(plaza_wpos, 0.5, 8.0)
                        .debug(|| "walk to plaza")
                        // ...then wait for some time before moving on
                        .then({
                            let wait_time = thread_rng().gen_range(10.0..30.0);
                            idle().repeat().stop_if(timeout(wait_time))
                                .debug(|| "wait at plaza")
                        })
                        .map(|_| ())
                        .boxed()
                } else {
                    // No plazas? :(
                    finish().boxed()
                }
            }))
        }
    })
    .debug(move || format!("villager at site {:?}", visiting_site))
}

fn think() -> impl Action {
    choose(|ctx| {
        if matches!(
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

// if !matches!(stages.front(), Some(TravelStage::IntraSite { .. })) {
//     let data = ctx.state.data();
//     if let Some((site2, site)) = npc
//         .current_site
//         .and_then(|current_site| data.sites.get(current_site))
//         .and_then(|site| site.world_site)
//         .and_then(|site| Some((get_site2(site)?, site)))
//     {
//         let end = site2.wpos_tile_pos(self.wpos.as_());
//         if let Some(path) = path_town(npc.wpos, site, ctx.index, |_|
// Some(end)) {             stages.push_front(TravelStage::IntraSite { path,
// site });         }
//     }
// }
