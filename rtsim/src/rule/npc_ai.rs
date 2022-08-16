use std::{collections::VecDeque, hash::BuildHasherDefault};

use crate::{
    data::{npc::PathData, Sites},
    event::OnTick,
    RtState, Rule, RuleError,
};
use common::{
    astar::{Astar, PathResult},
    path::Path,
    rtsim::{Profession, SiteId},
    store::Id,
    terrain::TerrainChunkSize, vol::RectVolSize,
};
use fxhash::FxHasher64;
use itertools::Itertools;
use rand::seq::IteratorRandom;
use vek::*;
use world::{
    civ::{self, Track},
    site::{Site as WorldSite, SiteKind},
    site2::{self, TileKind},
    IndexRef, World,
};

pub struct NpcAi;

const NEIGHBOURS: &[Vec2<i32>] = &[
    Vec2::new(1, 0),
    Vec2::new(0, 1),
    Vec2::new(-1, 0),
    Vec2::new(0, -1),
    Vec2::new(1, 1),
    Vec2::new(-1, 1),
    Vec2::new(-1, -1),
    Vec2::new(1, -1),
];
const CARDINALS: &[Vec2<i32>] = &[
    Vec2::new(1, 0),
    Vec2::new(0, 1),
    Vec2::new(-1, 0),
    Vec2::new(0, -1),
];

fn path_in_site(start: Vec2<i32>, end: Vec2<i32>, site: &site2::Site) -> PathResult<Vec2<i32>> {
    let heuristic = |tile: &Vec2<i32>| tile.as_::<f32>().distance(end.as_());
    let mut astar = Astar::new(
        100,
        start,
        &heuristic,
        BuildHasherDefault::<FxHasher64>::default(),
    );

    let transition = |a: &Vec2<i32>, b: &Vec2<i32>| {
        let distance = a.as_::<f32>().distance(b.as_());
        let a_tile = site.tiles.get(*a);
        let b_tile = site.tiles.get(*b);

        let terrain = match &b_tile.kind {
            TileKind::Empty => 5.0,
            TileKind::Hazard(_) => 20.0,
            TileKind::Field => 12.0,
            TileKind::Plaza | TileKind::Road { .. } => 1.0,

            TileKind::Building
            | TileKind::Castle
            | TileKind::Wall(_)
            | TileKind::Tower(_)
            | TileKind::Keep(_)
            | TileKind::Gate
            | TileKind::GnarlingFortification => 3.0,
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
                .unwrap_or(f32::INFINITY)
        } else if b_tile.is_building() && a_tile.is_road() {
            b_tile
                .plot
                .and_then(|plot| is_door_tile(plot, *b).then(|| 1.0))
                .unwrap_or(f32::INFINITY)
        } else if (a_tile.is_building() || b_tile.is_building()) && a_tile.plot != b_tile.plot {
            f32::INFINITY
        } else {
            1.0
        };

        distance * terrain * building
    };

    astar.poll(
        100,
        heuristic,
        |&tile| NEIGHBOURS.iter().map(move |c| tile + *c),
        transition,
        |tile| *tile == end,
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
        100,
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

    let path = astar.poll(100, heuristic, neighbors, transition, |site| *site == end);

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
            fn pop_first<T>(mut queue: VecDeque<T>) -> VecDeque<T> {
                queue.pop_front();
                queue
            }

            match path_in_site(start, end, site) {
                PathResult::Path(p) => Some(PathData {
                    end,
                    path: pop_first(p.nodes.into()),
                    repoll: false,
                }),
                PathResult::Exhausted(p) => Some(PathData {
                    end,
                    path: pop_first(p.nodes.into()),
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

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            let mut dynamic_rng = rand::thread_rng();
            for npc in data.npcs.values_mut() {
                npc.current_site = ctx.world.sim().get(npc.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_()).and_then(|chunk| {
                    data.sites.world_site_map.get(chunk.sites.first()?).copied()
                });
                
                if let Some(home_id) = npc.home {
                    if let Some((target, _)) = npc.target {
                        // Walk to the current target
                        if target.xy().distance_squared(npc.wpos.xy()) < 4.0 {
                            npc.target = None;
                        }
                    } else {
                        if let Some((ref mut path, site)) = npc.pathing.intrasite_path {
                            // If the npc walking in a site and want to reroll (because the path was
                            // exhausted.) to try to find a complete path.
                            if path.repoll {
                                npc.pathing.intrasite_path =
                                    path_town(npc.wpos, site, ctx.index, |_| Some(path.end))
                                        .map(|path| (path, site));
                            }
                        }
                        if let Some((ref mut path, site)) = npc.pathing.intrasite_path {
                            if let Some(next_tile) = path.path.pop_front() {
                                match &ctx.index.sites.get(site).kind {
                                    SiteKind::Refactor(site)
                                    | SiteKind::CliffTown(site)
                                    | SiteKind::DesertCity(site) => {
                                        // Set the target to the next node in the path.
                                        let wpos = site.tile_center_wpos(next_tile);
                                        let wpos = wpos.as_::<f32>().with_z(
                                            ctx.world.sim().get_alt_approx(wpos).unwrap_or(0.0),
                                        );

                                        npc.target = Some((wpos, 1.0));
                                    },
                                    _ => {},
                                }
                            } else {
                                // If the path is empty, we're done.
                                npc.pathing.intrasite_path = None;
                            }
                        } else if let Some((path, progress)) = {
                            // Check if we are done with this part of the inter site path.
                            if let Some((path, progress)) = &mut npc.pathing.intersite_path {
                                if let Some((track_id, _)) = path.path.front() {
                                    let track = ctx.world.civs().tracks.get(*track_id);
                                    if *progress >= track.path().len() {
                                        if path.repoll {
                                            // Repoll if last path wasn't complete.
                                            npc.pathing.intersite_path = path_towns(
                                                npc.current_site.unwrap(),
                                                path.end,
                                                &data.sites,
                                                ctx.world,
                                            );
                                        } else {
                                            // Otherwise just take the next in the calculated path.
                                            path.path.pop_front();
                                            *progress = 0;
                                        }
                                    }
                                }
                            }
                            &mut npc.pathing.intersite_path
                        } {
                            if let Some((track_id, reversed)) = path.path.front() {
                                let track = ctx.world.civs().tracks.get(*track_id);
                                let get_progress = |progress: usize| {
                                    if *reversed {
                                        track.path().len().wrapping_sub(progress + 1)
                                    } else {
                                        progress
                                    }
                                };

                                let transform_path_pos = |chunk_pos| {
                                    let chunk_wpos = TerrainChunkSize::center_wpos(chunk_pos);
                                    if let Some(pathdata) =
                                        ctx.world.sim().get_nearest_path(chunk_wpos)
                                    {
                                        pathdata.1.map(|e| e as i32)
                                    } else {
                                        chunk_wpos
                                    }
                                };

                                // Loop through and skip nodes that are inside a site, and use intra
                                // site path finding there instead.
                                let walk_path = loop {
                                    if let Some(chunk_pos) =
                                        track.path().nodes.get(get_progress(*progress))
                                    {
                                        if let Some((wpos, site_id, site)) =
                                            ctx.world.sim().get(*chunk_pos).and_then(|chunk| {
                                                let site_id = *chunk.sites.first()?;
                                                let wpos = transform_path_pos(*chunk_pos);
                                                match &ctx.index.sites.get(site_id).kind {
                                                    SiteKind::Refactor(site)
                                                    | SiteKind::CliffTown(site)
                                                    | SiteKind::DesertCity(site)
                                                        if !site.wpos_tile(wpos).is_empty() =>
                                                    {
                                                        Some((wpos, site_id, site))
                                                    },
                                                    _ => None,
                                                }
                                            })
                                        {
                                            if !site.wpos_tile(wpos).is_empty() {
                                                *progress += 1;
                                            } else {
                                                let end = site.wpos_tile_pos(wpos);
                                                npc.pathing.intrasite_path =
                                                    path_town(npc.wpos, site_id, ctx.index, |_| {
                                                        Some(end)
                                                    })
                                                    .map(|path| (path, site_id));
                                                break false;
                                            }
                                        } else {
                                            break true;
                                        }
                                    } else {
                                        break false;
                                    }
                                };

                                if walk_path {
                                    // Find the next wpos on the path.
                                    // NOTE: Consider not having this big gap between current
                                    // position and next. For better path finding. Maybe that would
                                    // mean having a float for progress.
                                    let wpos = transform_path_pos(
                                        track.path().nodes[get_progress(*progress)],
                                    );
                                    let wpos = wpos.as_::<f32>().with_z(
                                        ctx.world.sim().get_alt_approx(wpos).unwrap_or(0.0),
                                    );
                                    npc.target = Some((wpos, 1.0));
                                    *progress += 1;
                                }
                            } else {
                                npc.pathing.intersite_path = None;
                            }
                        } else {
                            if matches!(npc.profession, Some(Profession::Adventurer(_))) {
                                // If the npc is home, choose a random site to go to, otherwise go
                                // home.
                                if let Some(start) = npc.current_site {
                                    let end = if home_id == start {
                                        data.sites
                                            .keys()
                                            .filter(|site| *site != home_id)
                                            .choose(&mut dynamic_rng)
                                            .unwrap_or(home_id)
                                    } else {
                                        home_id
                                    };
                                    npc.pathing.intersite_path =
                                        path_towns(start, end, &data.sites, ctx.world);
                                }
                            } else {
                                // Choose a random plaza in the npcs home site (which should be the
                                // current here) to go to.
                                if let Some(home_id) =
                                    data.sites.get(home_id).and_then(|site| site.world_site)
                                {
                                    npc.pathing.intrasite_path =
                                        path_town(npc.wpos, home_id, ctx.index, |site| {
                                            Some(
                                                site.plots
                                                    [site.plazas().choose(&mut dynamic_rng)?]
                                                .root_tile(),
                                            )
                                        })
                                        .map(|path| (path, home_id));
                                }
                            }
                        }
                    }
                } else {
                    // TODO: Don't make homeless people walk around in circles
                    npc.target = Some((
                        npc.wpos
                            + Vec3::new(
                                ctx.event.time.0.sin() as f32 * 16.0,
                                ctx.event.time.0.cos() as f32 * 16.0,
                                0.0,
                            ),
                        1.0,
                    ));
                }
            }
        });

        Ok(Self)
    }
}
