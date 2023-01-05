use std::{collections::VecDeque, hash::BuildHasherDefault};

use crate::{
    ai::{casual, choose, finish, just, now, seq, urgent, watch, Action, NpcCtx},
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
    site2::{self, TileKind},
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
) -> Option<(PathData<(Id<Track>, bool), SiteId>, usize)> {
    match path_between_sites(start, end, sites, world) {
        PathResult::Exhausted(p) => Some((
            PathData {
                end,
                path: p.nodes.into(),
                repoll: true,
            },
            0,
        )),
        PathResult::Path(p) => Some((
            PathData {
                end,
                path: p.nodes.into(),
                repoll: false,
            },
            0,
        )),
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

fn idle() -> impl Action { just(|ctx| *ctx.controller = Controller::idle()) }

/// Try to walk toward a 3D position without caring for obstacles.
fn goto(wpos: Vec3<f32>, speed_factor: f32) -> impl Action {
    const STEP_DIST: f32 = 16.0;
    const GOAL_DIST: f32 = 2.0;

    just(move |ctx| {
        let rpos = wpos - ctx.npc.wpos;
        let len = rpos.magnitude();
        ctx.controller.goto = Some((
            ctx.npc.wpos + (rpos / len) * len.min(STEP_DIST),
            speed_factor,
        ));
    })
    .repeat()
    .stop_if(move |ctx| ctx.npc.wpos.xy().distance_squared(wpos.xy()) < GOAL_DIST.powi(2))
    .map(|_| {})
}

/// Try to walk toward a 2D position on the terrain without caring for
/// obstacles.
fn goto_2d(wpos2d: Vec2<f32>, speed_factor: f32) -> impl Action {
    const MIN_DIST: f32 = 2.0;

    now(move |ctx| {
        let wpos = wpos2d.with_z(ctx.world.sim().get_alt_approx(wpos2d.as_()).unwrap_or(0.0));
        goto(wpos, speed_factor)
    })
}

/// Try to travel to a site. Where practical, paths will be taken.
fn travel_to_site(tgt_site: SiteId) -> impl Action {
    now(move |ctx| {
        let sites = &ctx.state.data().sites;

        // If we're currently in a site, try to find a path to the target site via
        // tracks
        if let Some(current_site) = ctx.npc.current_site
            && let Some((mut tracks, _)) = path_towns(current_site, tgt_site, sites, ctx.world)
        {
            // For every track in the path we discovered between the sites...
            seq(tracks
                .path
                .into_iter()
                // ...traverse the nodes of that path.
                .map(|(track_id, reversed)| now(move |ctx| {
                    let track_len = ctx.world.civs().tracks.get(track_id).path().len();
                    // Tracks can be traversed backward (i.e: from end to beginning). Account for this.
                    seq(if reversed {
                        itertools::Either::Left((0..track_len).rev())
                    } else {
                        itertools::Either::Right(0..track_len)
                    }
                        .map(move |node_idx| now(move |ctx| {
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
                            goto_2d(node_wpos.as_(), 1.0)
                        })))
                })))
                .boxed()
        } else if let Some(site) = sites.get(tgt_site) {
            // If all else fails, just walk toward the target site in a straight line
            goto_2d(site.wpos.map(|e| e as f32 + 0.5), 1.0).boxed()
        } else {
            // If we can't find a way to get to the site at all, there's nothing more to be done
            finish().boxed()
        }
    })
}

// Seconds
fn timeout(ctx: &NpcCtx, time: f64) -> impl FnMut(&mut NpcCtx) -> bool + Clone + Send + Sync {
    let end = ctx.time.0 + time;
    move |ctx| ctx.time.0 > end
}

fn think() -> impl Action {
    choose(|ctx| {
        if matches!(ctx.npc.profession, Some(Profession::Adventurer(_))) {
            // Choose a random site that's fairly close by
            if let Some(tgt_site) = ctx
                .state
                .data()
                .sites
                .iter()
                .filter(|(site_id, site)| {
                    // TODO: faction.is_some() is used as a proxy for whether the site likely has
                    // paths, don't do this
                    site.faction.is_some()
                        && ctx.npc.current_site.map_or(true, |cs| *site_id != cs)
                        && thread_rng().gen_bool(0.25)
                })
                .min_by_key(|(_, site)| site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32)
                .map(|(site_id, _)| site_id)
            {
                casual(travel_to_site(tgt_site))
            } else {
                casual(finish())
            }
        } else if matches!(ctx.npc.profession, Some(Profession::Blacksmith)) {
            casual(idle())
        } else {
            casual(
                now(|ctx| goto(ctx.npc.wpos + Vec3::unit_x() * 10.0, 1.0))
                    .then(now(|ctx| goto(ctx.npc.wpos - Vec3::unit_x() * 10.0, 1.0)))
                    .repeat()
                    .stop_if(timeout(ctx, 10.0))
                    .then(now(|ctx| idle().repeat().stop_if(timeout(ctx, 5.0))))
                    .map(|_| {}),
            )
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
