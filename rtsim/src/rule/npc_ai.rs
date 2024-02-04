use std::{collections::VecDeque, hash::BuildHasherDefault};

use crate::{
    ai::{
        casual, choose, finish, important, just, now,
        predicate::{every_range, timeout, Chance, EveryRange, Predicate},
        seq, until, Action, NpcCtx, State,
    },
    data::{
        npc::{Brain, PathData, SimulationMode},
        ReportKind, Sentiment, Sites,
    },
    event::OnTick,
    RtState, Rule, RuleError,
};
use common::{
    astar::{Astar, PathResult},
    comp::{
        self, bird_large,
        compass::{Direction, Distance},
        dialogue::Subject,
        Content,
    },
    path::Path,
    rtsim::{Actor, ChunkResource, NpcInput, Profession, Role, SiteId},
    spiral::Spiral2d,
    store::Id,
    terrain::{CoordinateConversions, TerrainChunkSize},
    time::DayPeriod,
    util::Dir,
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
    site2::{self, plot::tavern, PlotKind, TileKind},
    util::NEIGHBORS,
    IndexRef, World,
};

/// How many ticks should pass between running NPC AI.
/// Note that this only applies to simulated NPCs: loaded NPCs have their AI
/// code run every tick. This means that AI code should be broadly
/// DT-independent.
const SIMULATED_TICK_SKIP: u64 = 10;

pub struct NpcAi;

const CARDINALS: &[Vec2<i32>] = &[
    Vec2::new(1, 0),
    Vec2::new(0, 1),
    Vec2::new(-1, 0),
    Vec2::new(0, -1),
];

#[derive(Clone)]
struct DefaultState {
    socialize_timer: EveryRange,
    move_home_timer: Chance<EveryRange>,
}

fn path_in_site(start: Vec2<i32>, end: Vec2<i32>, site: &site2::Site) -> PathResult<Vec2<i32>> {
    let heuristic = |tile: &Vec2<i32>, _: &Vec2<i32>| tile.as_::<f32>().distance(end.as_());
    let mut astar = Astar::new(1000, start, BuildHasherDefault::<FxHasher64>::default());

    let transition = |a: Vec2<i32>, b: Vec2<i32>| {
        let distance = a.as_::<f32>().distance(b.as_());
        let a_tile = site.tiles.get(a);
        let b_tile = site.tiles.get(b);

        let terrain = match &b_tile.kind {
            TileKind::Empty => 3.0,
            TileKind::Hazard(_) => 50.0,
            TileKind::Field => 8.0,
            TileKind::Plaza | TileKind::Road { .. } | TileKind::Path | TileKind::Bridge => 1.0,

            TileKind::Building
            | TileKind::Castle
            | TileKind::Wall(_)
            | TileKind::Tower(_)
            | TileKind::Keep(_)
            | TileKind::Gate
            | TileKind::AdletStronghold
            | TileKind::DwarvenMine
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
                .and_then(|plot| is_door_tile(plot, a).then_some(1.0))
                .unwrap_or(10000.0)
        } else if b_tile.is_building() && a_tile.is_road() {
            b_tile
                .plot
                .and_then(|plot| is_door_tile(plot, b).then_some(1.0))
                .unwrap_or(10000.0)
        } else if (a_tile.is_building() || b_tile.is_building()) && a_tile.plot != b_tile.plot {
            10000.0
        } else {
            1.0
        };

        distance * terrain + building
    };

    let neighbors = |tile: &Vec2<i32>| {
        let tile = *tile;
        CARDINALS.iter().map(move |c| {
            let n = tile + *c;
            (n, transition(tile, n))
        })
    };

    astar.poll(1000, heuristic, neighbors, |tile| {
        *tile == end || site.tiles.get_known(*tile).is_none()
    })
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

    let transition = |a: Id<civ::Site>, b: Id<civ::Site>| {
        world
            .civs()
            .track_between(a, b)
            .map(|(id, _)| world.civs().tracks.get(id).cost)
            .unwrap_or(f32::INFINITY)
    };
    let neighbors = |site: &Id<civ::Site>| {
        let site = *site;
        world
            .civs()
            .neighbors(site)
            .map(move |n| (n, transition(n, site)))
    };

    let path = astar.poll(250, heuristic, neighbors, |site| *site == end);

    path.map(|path| {
        let path = path
            .into_iter()
            .tuple_windows::<(_, _)>()
            // Since we get a, b from neighbors, track_between shouldn't return None.
            .filter_map(|(a, b)| world.civs().track_between(a, b))
            .collect();
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
            PathResult::Path(p, _c) => p.nodes,
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
        PathResult::Path(p, _c) => Some(PathData {
            end,
            path: p.nodes.into(),
            repoll: false,
        }),
        PathResult::Pending | PathResult::None(_) => None,
    }
}

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        // Keep track of the last `SIMULATED_TICK_SKIP` ticks, to know the deltatime
        // since the last tick we ran the npc.
        let mut last_ticks: VecDeque<_> = [1.0 / 30.0; SIMULATED_TICK_SKIP as usize]
            .into_iter()
            .collect();

        rtstate.bind::<Self, OnTick>(move |ctx| {
            last_ticks.push_front(ctx.event.dt);
            if last_ticks.len() >= SIMULATED_TICK_SKIP as usize {
                last_ticks.pop_back();
            }
            // Temporarily take the brains of NPCs out of their heads to appease the borrow
            // checker
            let mut npc_data = {
                let mut data = ctx.state.data_mut();
                data.npcs
                    .iter_mut()
                    // Don't run AI for dead NPCs
                    .filter(|(_, npc)| !npc.is_dead && !matches!(npc.role, Role::Vehicle))
                    // Don't run AI for simulated NPCs every tick
                    .filter(|(_, npc)| matches!(npc.mode, SimulationMode::Loaded) || (npc.seed as u64 + ctx.event.tick) % SIMULATED_TICK_SKIP == 0)
                    .map(|(npc_id, npc)| {
                        let controller = std::mem::take(&mut npc.controller);
                        let inbox = std::mem::take(&mut npc.inbox);
                        let sentiments = std::mem::take(&mut npc.sentiments);
                        let known_reports = std::mem::take(&mut npc.known_reports);
                        let brain = npc.brain.take().unwrap_or_else(|| Brain {
                            action: Box::new(think().repeat().with_state(DefaultState {
                                socialize_timer: every_range(15.0..30.0),
                                move_home_timer: every_range(400.0..2000.0).chance(0.5),
                            })),
                        });
                        (npc_id, controller, inbox, sentiments, known_reports, brain)
                    })
                    .collect::<Vec<_>>()
            };

            // The sum of the last `SIMULATED_TICK_SKIP` tick deltatimes is the deltatime since
            // simulated npcs ran this tick had their ai ran.
            let simulated_dt = last_ticks.iter().sum::<f32>();

            // Do a little thinking
            {
                let data = &*ctx.state.data();

                npc_data
                    .par_iter_mut()
                    .for_each(|(npc_id, controller, inbox, sentiments, known_reports, brain)| {
                        let npc = &data.npcs[*npc_id];

                        // Reset look_dir
                        controller.look_dir = None;

                        brain.action.tick(&mut NpcCtx {
                            state: ctx.state,
                            world: ctx.world,
                            index: ctx.index,
                            time_of_day: ctx.event.time_of_day,
                            time: ctx.event.time,
                            npc,
                            npc_id: *npc_id,
                            controller,
                            inbox,
                            known_reports,
                            sentiments,
                            dt: if matches!(npc.mode, SimulationMode::Loaded) {
                                ctx.event.dt
                            } else {
                                simulated_dt
                            },
                            rng: ChaChaRng::from_seed(thread_rng().gen::<[u8; 32]>()),
                        }, &mut ());
                    });
            }

            // Reinsert NPC brains
            let mut data = ctx.state.data_mut();
            for (npc_id, controller, inbox, sentiments, known_reports, brain) in npc_data {
                data.npcs[npc_id].controller = controller;
                data.npcs[npc_id].brain = Some(brain);
                data.npcs[npc_id].inbox = inbox;
                data.npcs[npc_id].sentiments = sentiments;
                data.npcs[npc_id].known_reports = known_reports;
            }
        });

        Ok(Self)
    }
}

fn idle<S: State>() -> impl Action<S> + Clone {
    just(|ctx, _| ctx.controller.do_idle()).debug(|| "idle")
}

/// Try to walk toward a 3D position without caring for obstacles.
fn goto<S: State>(wpos: Vec3<f32>, speed_factor: f32, goal_dist: f32) -> impl Action<S> {
    const STEP_DIST: f32 = 24.0;
    const WAYPOINT_DIST: f32 = 12.0;

    just(move |ctx, waypoint: &mut Option<Vec3<f32>>| {
        // If we're close to the next waypoint, complete it
        if waypoint.map_or(false, |waypoint: Vec3<f32>| {
            ctx.npc.wpos.xy().distance_squared(waypoint.xy()) < WAYPOINT_DIST.powi(2)
        }) {
            *waypoint = None;
        }

        // Get the next waypoint on the route toward the goal
        let waypoint = waypoint.get_or_insert_with(|| {
            let rpos = wpos - ctx.npc.wpos;
            let len = rpos.magnitude();
            let wpos = ctx.npc.wpos + (rpos / len) * len.min(STEP_DIST);

            wpos.with_z(ctx.world.sim().get_surface_alt_approx(wpos.xy().as_()))
        });

        ctx.controller.do_goto(*waypoint, speed_factor);
    })
    .repeat()
    .stop_if(move |ctx: &mut NpcCtx| {
        ctx.npc.wpos.xy().distance_squared(wpos.xy()) < goal_dist.powi(2)
    })
    .with_state(None)
    .debug(move || format!("goto {}, {}, {}", wpos.x, wpos.y, wpos.z))
    .map(|_, _| {})
}

/// Try to walk fly a 3D position following the terrain altitude at an offset
/// without caring for obstacles.
fn goto_flying<S: State>(
    wpos: Vec3<f32>,
    speed_factor: f32,
    goal_dist: f32,
    step_dist: f32,
    waypoint_dist: f32,
    height_offset: f32,
) -> impl Action<S> {
    just(move |ctx, waypoint: &mut Option<Vec3<f32>>| {
        // If we're close to the next waypoint, complete it
        if waypoint.map_or(false, |waypoint: Vec3<f32>| {
            ctx.npc.wpos.distance_squared(waypoint) < waypoint_dist.powi(2)
        }) {
            *waypoint = None;
        }

        // Get the next waypoint on the route toward the goal
        let waypoint = waypoint.get_or_insert_with(|| {
            let rpos = wpos - ctx.npc.wpos;
            let len = rpos.magnitude();
            let wpos = ctx.npc.wpos + (rpos / len) * len.min(step_dist);

            wpos.with_z(ctx.world.sim().get_surface_alt_approx(wpos.xy().as_()) + height_offset)
        });

        ctx.controller.do_goto(*waypoint, speed_factor);
    })
    .repeat()
    .boxed()
    .with_state(None)
    .stop_if(move |ctx: &mut NpcCtx| {
        ctx.npc.wpos.xy().distance_squared(wpos.xy()) < goal_dist.powi(2)
    })
    .debug(move || format!("goto {}, {}, {}", wpos.x, wpos.y, wpos.z))
    .map(|_, _| {})
}

/// Try to walk toward a 2D position on the surface without caring for
/// obstacles.
fn goto_2d<S: State>(wpos2d: Vec2<f32>, speed_factor: f32, goal_dist: f32) -> impl Action<S> {
    now(move |ctx, _| {
        let wpos = wpos2d.with_z(ctx.world.sim().get_surface_alt_approx(wpos2d.as_()));
        goto(wpos, speed_factor, goal_dist)
    })
}

/// Try to fly toward a 2D position following the terrain altitude at an offset
/// without caring for obstacles.
fn goto_2d_flying<S: State>(
    wpos2d: Vec2<f32>,
    speed_factor: f32,
    goal_dist: f32,
    step_dist: f32,
    waypoint_dist: f32,
    height_offset: f32,
) -> impl Action<S> {
    now(move |ctx, _| {
        let wpos =
            wpos2d.with_z(ctx.world.sim().get_surface_alt_approx(wpos2d.as_()) + height_offset);
        goto_flying(
            wpos,
            speed_factor,
            goal_dist,
            step_dist,
            waypoint_dist,
            height_offset,
        )
    })
}

fn traverse_points<S: State, F>(next_point: F, speed_factor: f32) -> impl Action<S>
where
    F: FnMut(&mut NpcCtx) -> Option<Vec2<f32>> + Clone + Send + Sync + 'static,
{
    until(move |ctx, next_point: &mut F| {
        let wpos = next_point(ctx)?;

        let wpos_site = |wpos: Vec2<f32>| {
            ctx.world
                .sim()
                .get(wpos.as_().wpos_to_cpos())
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
                    seq(path.into_iter().map(move |wpos| goto_2d(wpos, 1.0, 8.0))).then(goto_2d(
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
    .with_state(next_point)
}

/// Try to travel to a site. Where practical, paths will be taken.
fn travel_to_point<S: State>(wpos: Vec2<f32>, speed_factor: f32) -> impl Action<S> {
    now(move |ctx, _| {
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
fn travel_to_site<S: State>(tgt_site: SiteId, speed_factor: f32) -> impl Action<S> {
    now(move |ctx, _| {
        let sites = &ctx.state.data().sites;

        let site_wpos = sites.get(tgt_site).map(|site| site.wpos.as_());

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
            // Stop the NPC early if we're near the site to prevent huddling around the centre
            .stop_if(move |ctx: &mut NpcCtx| site_wpos.map_or(false, |site_wpos| ctx.npc.wpos.xy().distance_squared(site_wpos) < 16f32.powi(2)))
    })
        .debug(move || format!("travel_to_site {:?}", tgt_site))
        .map(|_, _| ())
}

fn talk_to<S: State>(tgt: Actor, _subject: Option<Subject>) -> impl Action<S> + Clone {
    now(move |ctx, _| {
        if matches!(tgt, Actor::Npc(_)) && ctx.rng.gen_bool(0.2) {
            // Cut off the conversation sometimes to avoid infinite conversations (but only
            // if the target is an NPC!) TODO: Don't special case this, have
            // some sort of 'bored of conversation' system
            idle().l()
        } else {
            // Mention nearby sites
            let comment = if ctx.rng.gen_bool(0.3)
                && let Some(current_site) = ctx.npc.current_site
                && let Some(current_site) = ctx.state.data().sites.get(current_site)
                && let Some(mention_site) = current_site.nearby_sites_by_size.choose(&mut ctx.rng)
                && let Some(mention_site) = ctx.state.data().sites.get(*mention_site)
                && let Some(mention_site_name) = mention_site.world_site
                    .map(|ws| ctx.index.sites.get(ws).name().to_string())
            {
                Content::localized_with_args("npc-speech-tell_site", [
                    ("site", Content::Plain(mention_site_name)),
                    ("dir", Direction::from_dir(mention_site.wpos.as_() - ctx.npc.wpos.xy()).localize_npc()),
                    ("dist", Distance::from_length(mention_site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32).localize_npc()),
                ])
            // Mention nearby monsters
            } else if ctx.rng.gen_bool(0.3)
                && let Some(monster) = ctx.state.data().npcs
                    .values()
                    .filter(|other| matches!(&other.role, Role::Monster))
                    .min_by_key(|other| other.wpos.xy().distance(ctx.npc.wpos.xy()) as i32)
            {
                Content::localized_with_args("npc-speech-tell_monster", [
                    ("body", monster.body.localize_npc()),
                    ("dir", Direction::from_dir(monster.wpos.xy() - ctx.npc.wpos.xy()).localize_npc()),
                    ("dist", Distance::from_length(monster.wpos.xy().distance(ctx.npc.wpos.xy()) as i32).localize_npc()),
                ])
            } else {
                ctx.npc.personality.get_generic_comment(&mut ctx.rng)
            };
            // TODO: Don't special-case players
            let wait = if matches!(tgt, Actor::Character(_)) {
                0.0
            } else {
                1.5
            };
            idle()
                .repeat()
                .stop_if(timeout(wait))
                .then(just(move |ctx, _| ctx.controller.say(tgt, comment.clone())))
                .r()
        }
    })
}

fn socialize() -> impl Action<EveryRange> + Clone {
    now(move |ctx, socialize: &mut EveryRange| {
        // Skip most socialising actions if we're not loaded
        if matches!(ctx.npc.mode, SimulationMode::Loaded) && socialize.should(ctx) {
            // Sometimes dance
            if ctx.rng.gen_bool(0.15) {
                return just(|ctx, _| ctx.controller.do_dance(None))
                    .repeat()
                    .stop_if(timeout(6.0))
                    .debug(|| "dancing")
                    .map(|_, _| ())
                    .l()
                    .l();
            // Talk to nearby NPCs
            } else if let Some(other) = ctx
                .state
                .data()
                .npcs
                .nearby(Some(ctx.npc_id), ctx.npc.wpos, 8.0)
                .choose(&mut ctx.rng)
            {
                return talk_to(other, None)
                    // After talking, wait for a while
                    .then(idle().repeat().stop_if(timeout(4.0)))
                    .map(|_, _| ())
                    .r().l();
            }
        }
        idle().r()
    })
}

fn adventure() -> impl Action<DefaultState> {
    choose(|ctx, _| {
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
                            | SiteKind::CoastalTown(_)
                            | SiteKind::DesertCity(_)
                    ),
                ) && ctx.npc.current_site.map_or(true, |cs| *site_id != cs)
                    && ctx.rng.gen_bool(0.25)
            })
            .min_by_key(|(_, site)| site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32)
            .map(|(site_id, _)| site_id)
        {
            let wait_time = if matches!(ctx.npc.profession(), Some(Profession::Merchant)) {
                60.0 * 15.0
            } else {
                60.0 * 3.0
            };
            let site_name = ctx.state.data().sites[tgt_site].world_site
                .map(|ws| ctx.index.sites.get(ws).name().to_string())
                .unwrap_or_default();
            // Travel to the site
            important(just(move |ctx, _| ctx.controller.say(None, Content::localized_with_args("npc-speech-moving_on", [("site", site_name.clone())])))
                          .then(travel_to_site(tgt_site, 0.6))
                          // Stop for a few minutes
                          .then(villager(tgt_site).repeat().stop_if(timeout(wait_time)))
                          .map(|_, _| ())
                          .boxed(),
            )
        } else {
            casual(finish().boxed())
        }
    })
    .debug(move || "adventure")
}

fn gather_ingredients<S: State>() -> impl Action<S> {
    just(|ctx, _| {
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

fn hunt_animals<S: State>() -> impl Action<S> {
    just(|ctx, _| ctx.controller.do_hunt_animals()).debug(|| "hunt_animals")
}

fn find_forest(ctx: &mut NpcCtx) -> Option<Vec2<f32>> {
    let chunk_pos = ctx.npc.wpos.xy().as_().wpos_to_cpos();
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

fn choose_plaza(ctx: &mut NpcCtx, site: SiteId) -> Option<Vec2<f32>> {
    ctx.state
        .data()
        .sites
        .get(site)
        .and_then(|site| ctx.index.sites.get(site.world_site?).site2())
        .and_then(|site2| {
            let plaza = &site2.plots[site2.plazas().choose(&mut ctx.rng)?];
            let tile = plaza
                .tiles()
                .choose(&mut ctx.rng)
                .unwrap_or_else(|| plaza.root_tile());
            Some(site2.tile_center_wpos(tile).as_())
        })
}

const WALKING_SPEED: f32 = 0.35;

fn villager(visiting_site: SiteId) -> impl Action<DefaultState> {
    choose(move |ctx, state: &mut DefaultState| {
        // Consider moving home if the home site gets too full
        if state.move_home_timer.should(ctx)
            && let Some(home) = ctx.npc.home
            && Some(home) == ctx.npc.current_site
            && let Some(home_pop_ratio) = ctx.state.data().sites.get(home)
                .and_then(|site| Some((site, ctx.index.sites.get(site.world_site?).site2()?)))
                .map(|(site, site2)| site.population.len() as f32 / site2.plots().len() as f32)
                // Only consider moving if the population is more than 1.5x the number of homes
                .filter(|pop_ratio| *pop_ratio > 1.5)
            && let Some(new_home) = ctx
                .state
                .data()
                .sites
                .iter()
                // Don't try to move to the site that's currently our home
                .filter(|(site_id, _)| Some(*site_id) != ctx.npc.home)
                // Only consider towns as potential homes
                .filter_map(|(site_id, site)| {
                    let site2 = match site.world_site.map(|ws| &ctx.index.sites.get(ws).kind) {
                        Some(SiteKind::Refactor(site2)
                            | SiteKind::CliffTown(site2)
                            | SiteKind::SavannahPit(site2)
                            | SiteKind::CoastalTown(site2)
                            | SiteKind::DesertCity(site2)) => site2,
                        _ => return None,
                    };
                    Some((site_id, site, site2))
                })
                // Only select sites that are less densely populated than our own
                .filter(|(_, site, site2)| (site.population.len() as f32 / site2.plots().len() as f32) < home_pop_ratio)
                // Find the closest of the candidate sites
                .min_by_key(|(_, site, _)| site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32)
                .map(|(site_id, _, _)| site_id)
        {
            let site_name = ctx.state.data().sites[new_home].world_site
                .map(|ws| ctx.index.sites.get(ws).name().to_string());
            return important(just(move |ctx, _| {
                if let Some(site_name) = &site_name {
                    ctx.controller.say(None, Content::localized_with_args("npc-speech-migrating", [("site", site_name.clone())]))
                }
            })
                .then(travel_to_site(new_home, 0.5))
                .then(just(move |ctx, _| ctx.controller.set_new_home(new_home))));
        }
        let day_period = DayPeriod::from(ctx.time_of_day.0);
        let is_weekend = ctx.time_of_day.day() as u64 % 6 == 0;
        if day_period.is_dark()
            && !matches!(ctx.npc.profession(), Some(Profession::Guard))
        {
            return important(
                now(move |ctx, _| {
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
                        just(|ctx, _| {
                            ctx.controller
                                .say(None, Content::localized("npc-speech-night_time"))
                        })
                        .then(travel_to_point(house_wpos, 0.65))
                        .debug(|| "walk to house")
                        .then(socialize().repeat().map_state(|state: &mut DefaultState| &mut state.socialize_timer).debug(|| "wait in house"))
                        .stop_if(|ctx: &mut NpcCtx| DayPeriod::from(ctx.time_of_day.0).is_light())
                        .then(just(|ctx, _| {
                            ctx.controller
                                .say(None, Content::localized("npc-speech-day_time"))
                        }))
                        .map(|_, _| ())
                        .boxed()
                    } else {
                        finish().boxed()
                    }
                })
                .debug(|| "find somewhere to sleep"),
            );
        }
        // Go do something fun on evenings and holidays, or on random days.
        else if
            // Ain't no rest for the wicked 
            !matches!(ctx.npc.profession(), Some(Profession::Guard))
            && (matches!(day_period, DayPeriod::Evening) || is_weekend || ctx.rng.gen_bool(0.05)) {
            let mut fun_stuff = Vec::new();

            if let Some(ws_id) = ctx.state.data().sites[visiting_site].world_site
                && let Some(ws) = ctx.index.sites.get(ws_id).site2() {
                if let Some(arena) = ws.plots().find_map(|p| match p.kind() { PlotKind::DesertCityArena(a) => Some(a), _ => None}) {
                    let wait_time = ctx.rng.gen_range(100.0..300.0);
                    // We don't use Z coordinates for seats because they are complicated to calculate from the Ramp procedural generation
                    // and using goto_2d seems to work just fine. However it also means that NPC will never go seat on the stands
                    // on the first floor of the arena. This is a compromise that was made because in the current arena procedural generation
                    // there is also no pathways to the stands on the first floor for NPCs.
                    let arena_center = Vec3::new(arena.center.x, arena.center.y, arena.base).as_::<f32>();
                    let stand_dist = arena.stand_dist as f32;
                    let seat_var_width = ctx.rng.gen_range(0..arena.stand_width) as f32;
                    let seat_var_length = ctx.rng.gen_range(-arena.stand_length..arena.stand_length) as f32;
                    // Select a seat on one of the 4 arena stands
                    let seat = match ctx.rng.gen_range(0..4) {
                        0 => Vec3::new(arena_center.x - stand_dist + seat_var_width, arena_center.y + seat_var_length, arena_center.z),
                        1 => Vec3::new(arena_center.x + stand_dist - seat_var_width, arena_center.y + seat_var_length, arena_center.z),
                        2 => Vec3::new(arena_center.x + seat_var_length, arena_center.y - stand_dist + seat_var_width, arena_center.z),
                        _ => Vec3::new(arena_center.x + seat_var_length, arena_center.y + stand_dist - seat_var_width, arena_center.z),
                    };
                    let look_dir = Dir::from_unnormalized(arena_center - seat);
                    // Walk to an arena seat, cheer, sit and dance
                    let action = casual(just(move |ctx, _| ctx.controller.say(None, Content::localized("npc-speech-arena")))
                            .then(goto_2d(seat.xy(), 0.6, 1.0).debug(|| "go to arena"))
                            // Turn toward the centre of the arena and watch the action!
                            .then(choose(move |ctx, _| if ctx.rng.gen_bool(0.3) {
                                casual(just(move |ctx,_| ctx.controller.do_cheer(look_dir)).repeat().stop_if(timeout(5.0)))
                            } else if ctx.rng.gen_bool(0.15) {
                                casual(just(move |ctx,_| ctx.controller.do_dance(look_dir)).repeat().stop_if(timeout(5.0)))
                            } else {
                                casual(just(move |ctx,_| ctx.controller.do_sit(look_dir, None)).repeat().stop_if(timeout(15.0)))
                            })
                                .repeat()
                                .stop_if(timeout(wait_time)))
                            .map(|_, _| ())
                            .boxed());
                    fun_stuff.push(action);
                }
                if let Some(tavern) = ws.plots().filter_map(|p| match p.kind() {  PlotKind::Tavern(a) => Some(a), _ => None }).choose(&mut ctx.rng) {
                    let wait_time = ctx.rng.gen_range(100.0..300.0);

                    let (stage_aabr, stage_z) = tavern.rooms.values().flat_map(|room| {
                        room.details.iter().filter_map(|detail| match detail {
                            tavern::Detail::Stage { aabr } => Some((*aabr, room.bounds.min.z + 1)),
                            _ => None,
                        })
                    }).choose(&mut ctx.rng).unwrap_or((tavern.bounds, tavern.door_wpos.z));

                    let bar_pos = tavern.rooms.values().flat_map(|room|
                        room.details.iter().filter_map(|detail| match detail {
                            tavern::Detail::Bar { aabr } => {
                                let side = site2::util::Dir::from_vec2(room.bounds.center().xy() - aabr.center());
                                let pos = side.select_aabr_with(*aabr, aabr.center()) + side.to_vec2();

                                Some(pos.with_z(room.bounds.min.z))
                            }
                            _ => None,
                        })
                    ).choose(&mut ctx.rng).unwrap_or(stage_aabr.center().with_z(stage_z));

                    // Pick a chair that is theirs for the stay
                    let chair_pos = tavern.rooms.values().flat_map(|room| {
                        let z = room.bounds.min.z;
                        room.details.iter().filter_map(move |detail| match detail {
                            tavern::Detail::Table { pos, chairs } => Some(chairs.into_iter().map(move |dir| pos.with_z(z) + dir.to_vec2())),
                            _ => None,
                        })
                        .flatten()
                    }
                    ).choose(&mut ctx.rng)
                    // This path is possible, but highly unlikely.
                    .unwrap_or(bar_pos);

                    let stage_aabr = stage_aabr.as_::<f32>();
                    let stage_z = stage_z as f32;

                    let action = casual(travel_to_point(tavern.door_wpos.xy().as_() + 0.5, 0.8).then(choose(move |ctx, (last_action, _)| {
                            let action = [0, 1, 2].into_iter().filter(|i| *last_action != Some(*i)).choose(&mut ctx.rng).expect("We have at least 2 elements");
                            let socialize = socialize().map_state(|(_, timer)| timer).repeat();
                            match action {
                                // Go and dance on a stage.
                                0 => {
                                    casual(now(move |ctx, (last_action, _)| {
                                        *last_action = Some(action);
                                        goto(stage_aabr.min.map2(stage_aabr.max, |a, b| ctx.rng.gen_range(a..b)).with_z(stage_z), WALKING_SPEED, 1.0)
                                    })
                                    .then(just(move |ctx,_| ctx.controller.do_dance(None)).repeat().stop_if(timeout(ctx.rng.gen_range(20.0..30.0))))
                                    .map(|_, _| ())
                                    )
                                },
                                // Go and sit at a table.
                                1 => {
                                    casual(
                                        now(move |ctx, (last_action, _)| {
                                            *last_action = Some(action);
                                            goto(chair_pos.as_() + 0.5, WALKING_SPEED, 1.0).then(just(move |ctx, _| ctx.controller.do_sit(None, Some(chair_pos)))).then(socialize.clone().stop_if(timeout(ctx.rng.gen_range(30.0..60.0)))).map(|_, _| ())
                                        })
                                    )
                                },
                                // Go to the bar.
                                _ => {
                                    casual(
                                        now(move |ctx, (last_action, _)| {
                                            *last_action = Some(action);
                                            goto(bar_pos.as_() + 0.5, WALKING_SPEED, 1.0).then(socialize.clone().stop_if(timeout(ctx.rng.gen_range(10.0..25.0)))).map(|_, _| ())
                                        })
                                    )
                                },
                            }
                        })
                        .with_state((None::<u32>, every_range(5.0..10.0)))
                        .repeat()
                        .stop_if(timeout(wait_time)))
                        .map(|_, _| ())
                        .boxed()
                    );

                    fun_stuff.push(action);
                }
            }


            if !fun_stuff.is_empty() {
                let i = ctx.rng.gen_range(0..fun_stuff.len());
                return fun_stuff.swap_remove(i);
            }
        }
        // Villagers with roles should perform those roles
        else if matches!(ctx.npc.profession(), Some(Profession::Herbalist)) && ctx.rng.gen_bool(0.8)
        {
            if let Some(forest_wpos) = find_forest(ctx) {
                return casual(
                    travel_to_point(forest_wpos, 0.5)
                        .debug(|| "walk to forest")
                        .then({
                            let wait_time = ctx.rng.gen_range(10.0..30.0);
                            gather_ingredients().repeat().stop_if(timeout(wait_time))
                        })
                        .map(|_, _| ()),
                );
            }
        } else if matches!(ctx.npc.profession(), Some(Profession::Hunter)) && ctx.rng.gen_bool(0.8) {
            if let Some(forest_wpos) = find_forest(ctx) {
                return casual(
                    just(|ctx, _| {
                        ctx.controller
                            .say(None, Content::localized("npc-speech-start_hunting"))
                    })
                    .then(travel_to_point(forest_wpos, 0.75))
                    .debug(|| "walk to forest")
                    .then({
                        let wait_time = ctx.rng.gen_range(30.0..60.0);
                        hunt_animals().repeat().stop_if(timeout(wait_time))
                    })
                    .map(|_, _| ()),
                );
            }
        } else if matches!(ctx.npc.profession(), Some(Profession::Guard)) && ctx.rng.gen_bool(0.7) {
            if let Some(plaza_wpos) = choose_plaza(ctx, visiting_site) {
                return casual(
                    travel_to_point(plaza_wpos, 0.4)
                        .debug(|| "patrol")
                        .interrupt_with(move |ctx, _| {
                            if ctx.rng.gen_bool(0.0003) {
                                Some(just(move |ctx, _| {
                                    ctx.controller
                                        .say(None, Content::localized("npc-speech-guard_thought"))
                                }))
                            } else {
                                None
                            }
                        })
                        .map(|_, _| ()),
                );
            }
        } else if matches!(ctx.npc.profession(), Some(Profession::Merchant)) && ctx.rng.gen_bool(0.8)
        {
            return casual(
                just(|ctx, _| {
                    // Try to direct our speech at nearby actors, if there are any
                    let (target, phrase) = if ctx.rng.gen_bool(0.3) && let Some(other) = ctx
                        .state
                        .data()
                        .npcs
                        .nearby(Some(ctx.npc_id), ctx.npc.wpos, 8.0)
                        .choose(&mut ctx.rng)
                    {
                        (Some(other), "npc-speech-merchant_sell_directed")
                    } else {
                        // Otherwise, resort to generic expressions
                        (None, "npc-speech-merchant_sell_undirected")
                    };

                    ctx.controller.say(target, Content::localized(phrase));
                })
                .then(idle().repeat().stop_if(timeout(8.0)))
                .repeat()
                .stop_if(timeout(60.0))
                .debug(|| "sell wares")
                .map(|_, _| ()),
            );
        }

        // If nothing else needs doing, walk between plazas and socialize
        casual(now(move |ctx, _| {
            // Choose a plaza in the site we're visiting to walk to
            if let Some(plaza_wpos) = choose_plaza(ctx, visiting_site) {
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
                    .map_state(|state: &mut DefaultState| &mut state.socialize_timer)
                    .stop_if(timeout(ctx.rng.gen_range(30.0..90.0)))
                    .debug(|| "wait at plaza"))
                .map(|_, _| ())
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

fn pilot<S: State>(ship: common::comp::ship::Body) -> impl Action<S> {
    // Travel between different towns in a straight line
    now(move |ctx, _| {
        let data = &*ctx.state.data();
        let station_wpos = data
            .sites
            .iter()
            .filter(|(id, _)| Some(*id) != ctx.npc.current_site)
            .filter_map(|(_, site)| ctx.index.sites.get(site.world_site?).site2())
            .flat_map(|site| {
                site.plots()
                    .filter(|plot| matches!(plot.kind(), PlotKind::AirshipDock(_)))
                    .map(|plot| site.tile_center_wpos(plot.root_tile()))
            })
            .choose(&mut ctx.rng);
        if let Some(station_wpos) = station_wpos {
            Either::Right(
                goto_2d_flying(
                    station_wpos.as_(),
                    1.0,
                    50.0,
                    150.0,
                    110.0,
                    ship.flying_height(),
                )
                .then(goto_2d_flying(
                    station_wpos.as_(),
                    1.0,
                    10.0,
                    32.0,
                    16.0,
                    30.0,
                )),
            )
        } else {
            Either::Left(finish())
        }
    })
    .repeat()
    .map(|_, _| ())
}

fn captain<S: State>() -> impl Action<S> {
    // For now just randomly travel the sea
    now(|ctx, _| {
        let chunk = ctx.npc.wpos.xy().as_().wpos_to_cpos();
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
    .map(|_, _| ())
}

fn check_inbox<S: State>(ctx: &mut NpcCtx) -> Option<impl Action<S>> {
    loop {
        match ctx.inbox.pop_front() {
            Some(NpcInput::Report(report_id)) if !ctx.known_reports.contains(&report_id) => {
                #[allow(clippy::single_match)]
                match ctx.state.data().reports.get(report_id).map(|r| r.kind) {
                    Some(ReportKind::Death { killer, actor, .. })
                        if matches!(&ctx.npc.role, Role::Civilised(_)) =>
                    {
                        // TODO: Don't report self
                        let phrase = if let Some(killer) = killer {
                            // TODO: For now, we don't make sentiment changes if the killer was an
                            // NPC because NPCs can't hurt one-another.
                            // This should be changed in the future.
                            if !matches!(killer, Actor::Npc(_)) {
                                // TODO: Don't hard-code sentiment change
                                let mut change = -0.7;
                                if ctx.sentiments.toward(actor).is(Sentiment::ENEMY) {
                                    // Like the killer if we have negative sentiment towards the
                                    // killed.
                                    change *= -1.0;
                                }
                                ctx.sentiments
                                    .toward_mut(killer)
                                    .change_by(change, Sentiment::VILLAIN);
                            }

                            // This is a murder of a player. Feel bad for the player and stop
                            // attacking them.
                            if let Actor::Character(_) = actor {
                                ctx.sentiments
                                    .toward_mut(actor)
                                    .limit_below(Sentiment::ENEMY)
                            }

                            if ctx.sentiments.toward(actor).is(Sentiment::ENEMY) {
                                "npc-speech-witness_enemy_murder"
                            } else {
                                "npc-speech-witness_murder"
                            }
                        } else {
                            "npc-speech-witness_death"
                        };
                        ctx.known_reports.insert(report_id);
                        break Some(
                            just(move |ctx, _| {
                                ctx.controller.say(killer, Content::localized(phrase))
                            })
                            .l(),
                        );
                    },
                    Some(ReportKind::Death { .. }) => {}, // We don't care about death
                    None => {},                           // Stale report, ignore
                }
            },
            Some(NpcInput::Report(_)) => {}, // Reports we already know of are ignored
            Some(NpcInput::Interaction(by, subject)) => break Some(talk_to(by, Some(subject)).r()),
            None => break None,
        }
    }
}

fn check_for_enemies<S: State>(ctx: &mut NpcCtx) -> Option<impl Action<S>> {
    // TODO: Instead of checking all nearby actors every tick, it would be more
    // effective to have the actor grid generate a per-tick diff so that we only
    // need to check new actors in the local area. Be careful though:
    // implementing this means accounting for changes in sentiment (that could
    // suddenly make a nearby actor an enemy) as well as variable NPC tick
    // rates!
    ctx.state
        .data()
        .npcs
        .nearby(Some(ctx.npc_id), ctx.npc.wpos, 24.0)
        .find(|actor| ctx.sentiments.toward(*actor).is(Sentiment::ENEMY))
        .map(|enemy| just(move |ctx, _| ctx.controller.attack(enemy)))
}

fn react_to_events<S: State>(ctx: &mut NpcCtx, _: &mut S) -> Option<impl Action<S>> {
    check_inbox::<S>(ctx)
        .map(|action| action.boxed())
        .or_else(|| check_for_enemies(ctx).map(|action| action.boxed()))
}

fn humanoid() -> impl Action<DefaultState> {
    choose(|ctx, _| {
        if let Some(riding) = &ctx.state.data().npcs.mounts.get_mount_link(ctx.npc_id) {
            if riding.is_steering {
                if let Some(vehicle) = ctx.state.data().npcs.get(riding.mount) {
                    match vehicle.body {
                        comp::Body::Ship(
                            body @ comp::ship::Body::DefaultAirship
                            | body @ comp::ship::Body::AirBalloon,
                        ) => important(pilot(body)),
                        comp::Body::Ship(
                            comp::ship::Body::SailBoat | comp::ship::Body::Galleon,
                        ) => important(captain()),
                        _ => casual(idle()),
                    }
                } else {
                    casual(finish())
                }
            } else {
                important(
                    socialize().map_state(|state: &mut DefaultState| &mut state.socialize_timer),
                )
            }
        } else {
            let action = if matches!(
                ctx.npc.profession(),
                Some(Profession::Adventurer(_) | Profession::Merchant)
            ) {
                adventure().l().l()
            } else if let Some(home) = ctx.npc.home {
                villager(home).r().l()
            } else {
                idle().r() // Homeless
            };

            casual(action.interrupt_with(react_to_events))
        }
    })
}

fn bird_large() -> impl Action<DefaultState> {
    now(|ctx, bearing: &mut Vec2<f32>| {
        *bearing = bearing
            .map(|e| e + ctx.rng.gen_range(-0.1..0.1))
            .try_normalized()
            .unwrap_or_default();
        let bearing_dist = 15.0;
        let mut pos = ctx.npc.wpos.xy() + *bearing * bearing_dist;
        let is_deep_water =
            matches!(ctx.npc.body, common::comp::Body::BirdLarge(b) if matches!(b.species, bird_large::Species::SeaWyvern))
                || ctx
                .world
                .sim()
                .get(pos.as_().wpos_to_cpos())
                .map_or(true, |c| {
                    c.alt - c.water_alt < -120.0 && (c.river.is_ocean() || c.river.is_lake())
                });
        if is_deep_water {
            *bearing *= -1.0;
            pos = ctx.npc.wpos.xy() + *bearing * bearing_dist;
        };
        // when high tree_density fly high, otherwise fly low-mid
        let npc_pos = ctx.npc.wpos.xy();
        let trees = ctx
            .world
            .sim()
            .get(npc_pos.as_().wpos_to_cpos())
            .map_or(false, |c| c.tree_density > 0.1);
        let height_factor = if trees {
            2.0
        } else {
            ctx.rng.gen_range(0.4..0.9)
        };

        let data = ctx.state.data();
        // without destination site fly to next waypoint
        let mut dest_site = pos;
        if let Some(home) = ctx.npc.home {
            let is_home = ctx.npc.current_site.map_or(false, |site| home == site);
            if is_home {
                if let Some((id, _)) = data
                    .sites
                    .iter()
                    .filter(|(id, site)| {
                        *id != home
                            && site.world_site.map_or(false, |site| {
                            match ctx.npc.body {
                                common::comp::Body::BirdLarge(b) => match b.species {
                                    bird_large::Species::SeaWyvern => matches!(&ctx.index.sites.get(site).kind, SiteKind::ChapelSite(_)),
                                    bird_large::Species::FrostWyvern => matches!(&ctx.index.sites.get(site).kind, SiteKind::Adlet(_)),
                                    bird_large::Species::WealdWyvern => matches!(&ctx.index.sites.get(site).kind, SiteKind::GiantTree(_)),
                                    _ => matches!(&ctx.index.sites.get(site).kind, SiteKind::Dungeon(_)),
                                },
                                _ => matches!(&ctx.index.sites.get(site).kind, SiteKind::Dungeon(_)),
                            }
                        })
                    })
                    /*choose closest destination:
                    .min_by_key(|(_, site)| site.wpos.as_().distance(npc_pos) as i32)*/
                //choose random destination:
                .choose(&mut ctx.rng)
                {
                    ctx.controller.set_new_home(id)
                }
            } else if let Some(site) = data.sites.get(home) {
                dest_site = site.wpos.as_::<f32>()
            }
        }
        goto_2d_flying(
            pos,
            0.2,
            bearing_dist,
            8.0,
            8.0,
            ctx.npc.body.flying_height() * height_factor,
        )
            // If we are too far away from our waypoint position we can stop since we aren't going to a specific place.
            // If waypoint position is further away from destination site find a new waypoint
            .stop_if(move |ctx: &mut NpcCtx| {
                ctx.npc.wpos.xy().distance_squared(pos) > (bearing_dist + 5.0).powi(2)
                    || dest_site.distance_squared(pos) > dest_site.distance_squared(npc_pos)
            })
            // If waypoint position wasn't reached within 10 seconds we're probably stuck and need to find a new waypoint.
            .stop_if(timeout(10.0))
            .debug({
                let bearing = *bearing;
                move || format!("Moving with a bearing of {:?}", bearing)
            })
    })
        .repeat()
        .with_state(Vec2::<f32>::zero())
        .map(|_, _| ())
}

fn monster() -> impl Action<DefaultState> {
    now(|ctx, bearing: &mut Vec2<f32>| {
        *bearing = bearing
            .map(|e| e + ctx.rng.gen_range(-0.1..0.1))
            .try_normalized()
            .unwrap_or_default();
        let bearing_dist = 24.0;
        let mut pos = ctx.npc.wpos.xy() + *bearing * bearing_dist;
        let is_deep_water = ctx
            .world
            .sim()
            .get(pos.as_().wpos_to_cpos())
            .map_or(true, |c| {
                c.alt - c.water_alt < -10.0 && (c.river.is_ocean() || c.river.is_lake())
            });
        if !is_deep_water {
            goto_2d(pos, 0.7, 8.0)
        } else {
            *bearing *= -1.0;

            pos = ctx.npc.wpos.xy() + *bearing * 24.0;

            goto_2d(pos, 0.7, 8.0)
        }
        // If we are too far away from our goal position we can stop since we aren't going to a specific place.
        .stop_if(move |ctx: &mut NpcCtx| {
            ctx.npc.wpos.xy().distance_squared(pos) > (bearing_dist + 5.0).powi(2)
        })
        .debug({
            let bearing = *bearing;
            move || format!("Moving with a bearing of {:?}", bearing)
        })
    })
    .repeat()
    .with_state(Vec2::<f32>::zero())
    .map(|_, _| ())
}

fn think() -> impl Action<DefaultState> {
    now(|ctx, _| match ctx.npc.body {
        common::comp::Body::Humanoid(_) => humanoid().l().l().l(),
        common::comp::Body::BirdLarge(_) => bird_large().r().l().l(),
        _ => match &ctx.npc.role {
            Role::Civilised(_) => socialize()
                .map_state(|state: &mut DefaultState| &mut state.socialize_timer)
                .l()
                .r()
                .l(),
            Role::Monster => monster().r().r().l(),
            Role::Wild => idle().r(),
            Role::Vehicle => idle().r(),
        },
    })
}
