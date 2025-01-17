use super::*;

fn path_in_site(start: Vec2<i32>, end: Vec2<i32>, site: &site2::Site) -> PathResult<Vec2<i32>> {
    let heuristic = |tile: &Vec2<i32>| tile.as_::<f32>().distance(end.as_());
    let mut astar = Astar::new(1_000, start, BuildHasherDefault::<FxHasher64>::default());

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
        let is_door_tile =
            |plot: Id<site2::Plot>, tile: Vec2<i32>| match site.plot(plot).kind().meta() {
                Some(PlotKindMeta::House { door_tile }) => door_tile == tile,
                Some(PlotKindMeta::Workshop { door_tile }) => {
                    door_tile.is_none_or(|door_tile| door_tile == tile)
                },
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

        const CARDINALS: &[Vec2<i32>] = &[
            Vec2::new(1, 0),
            Vec2::new(0, 1),
            Vec2::new(-1, 0),
            Vec2::new(0, -1),
        ];

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
    let heuristic = |site: &Id<civ::Site>| get_site(site).center.as_().distance(end_pos);

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

fn path_between_towns(
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

// Actions

/// Try to walk toward a 3D position without caring for obstacles.
pub fn goto<S: State>(wpos: Vec3<f32>, speed_factor: f32, goal_dist: f32) -> impl Action<S> {
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

pub fn follow_actor<S: State>(actor: Actor, distance: f32) -> impl Action<S> {
    // const STEP_DIST: f32 = 30.0;
    just(move |ctx, _| {
        if let Some(tgt_wpos) = util::locate_actor(ctx, actor)
            && let dist_sqr = tgt_wpos.xy().distance_squared(ctx.npc.wpos.xy())
            && dist_sqr > distance.powi(2)
        {
            // // Don't try to path too far in one go
            // let tgt_wpos = if dist_sqr > STEP_DIST.powi(2) {
            //     let tgt_wpos_2d = ctx.npc.wpos.xy() + (tgt_wpos -
            // ctx.npc.wpos).xy().normalized() * STEP_DIST;     tgt_wpos_2d.
            // with_z(ctx.world.sim().get_surface_alt_approx(tgt_wpos_2d.as_()))
            // } else {
            //     tgt_wpos
            // };
            ctx.controller.do_goto(tgt_wpos, 1.0);
        } else {
            ctx.controller.do_idle();
        }
    })
    .repeat()
    .debug(move || format!("Following actor {actor:?}"))
    .map(|_, _| ())
}

pub fn goto_actor<S: State>(actor: Actor, distance: f32) -> impl Action<S> {
    follow_actor(actor, distance)
        .stop_if(move |ctx: &mut NpcCtx| {
            if let Some(wpos) = util::locate_actor(ctx, actor) {
                wpos.xy().distance_squared(ctx.npc.wpos.xy()) < distance.powi(2)
            } else {
                false
            }
        })
        .map(|_, _| ())
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
    .debug(move || {
        format!(
            "goto flying ({}, {}, {}), goal dist {}",
            wpos.x, wpos.y, wpos.z, goal_dist
        )
    })
    .map(|_, _| {})
}

/// Try to walk toward a 2D position on the surface without caring for
/// obstacles.
pub fn goto_2d<S: State>(wpos2d: Vec2<f32>, speed_factor: f32, goal_dist: f32) -> impl Action<S> {
    now(move |ctx, _| {
        let wpos = wpos2d.with_z(ctx.world.sim().get_surface_alt_approx(wpos2d.as_()));
        goto(wpos, speed_factor, goal_dist).debug(move || {
            format!(
                "goto 2d ({}, {}), z {}, goal dist {}",
                wpos2d.x, wpos2d.y, wpos.z, goal_dist
            )
        })
    })
}

/// Try to fly toward a 2D position following the terrain altitude at an offset
/// without caring for obstacles.
pub fn goto_2d_flying<S: State>(
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
        .debug(move || {
            format!(
                "goto 2d flying ({}, {}), goal dist {}",
                wpos2d.x, wpos2d.y, goal_dist
            )
        })
    })
}

fn traverse_points<S: State, F>(next_point: F, speed_factor: f32) -> impl Action<S>
where
    F: FnMut(&mut NpcCtx) -> Option<Vec2<f32>> + Clone + Send + Sync + 'static,
{
    until(move |ctx, next_point: &mut F| {
        // Pick next waypoint, return if path ended
        let Some(wpos) = next_point(ctx) else {
            return ControlFlow::Break(());
        };

        let wpos_site = |wpos: Vec2<f32>| {
            ctx.world
                .sim()
                .get(wpos.as_().wpos_to_cpos())
                .and_then(|chunk| chunk.sites.first().copied())
        };

        let wpos_sites_contain = |wpos: Vec2<f32>, site: Id<world::site::Site>| {
            ctx.world
                .sim()
                .get(wpos.as_().wpos_to_cpos())
                .map(|chunk| chunk.sites.contains(&site))
                .unwrap_or(false)
        };

        let npc_wpos = ctx.npc.wpos;

        // If we're traversing within a site, do intra-site pathfinding
        if let Some(site) = wpos_site(wpos) {
            let mut site_exit = wpos;
            while let Some(next) = next_point(ctx).filter(|next| wpos_sites_contain(*next, site)) {
                site_exit = next;
            }

            // Navigate through the site to the site exit
            if let Some(path) = path_site(wpos, site_exit, site, ctx.index) {
                ControlFlow::Continue(Either::Left(
                    seq(path.into_iter().map(move |wpos| goto_2d(wpos, 1.0, 8.0))).then(goto_2d(
                        site_exit,
                        speed_factor,
                        8.0,
                    )),
                ))
            } else {
                // No intra-site path found, just attempt to move towards the exit node
                ControlFlow::Continue(Either::Right(
                    goto_2d(site_exit, speed_factor, 8.0)
                        .debug(move || {
                            format!(
                                "direct from {}, {}, ({}) to site exit at {}, {}",
                                npc_wpos.x, npc_wpos.y, npc_wpos.z, site_exit.x, site_exit.y
                            )
                        })
                        .boxed(),
                ))
            }
        } else {
            // We're in the middle of a road, just go to the next waypoint
            ControlFlow::Continue(Either::Right(
                goto_2d(wpos, speed_factor, 8.0)
                    .debug(move || {
                        format!(
                            "from {}, {}, ({}) to the next waypoint at {}, {}",
                            npc_wpos.x, npc_wpos.y, npc_wpos.z, wpos.x, wpos.y
                        )
                    })
                    .boxed(),
            ))
        }
    })
    .with_state(next_point)
    .debug(|| "traverse points")
}

/// Try to travel to a site. Where practical, paths will be taken.
pub fn travel_to_point<S: State>(wpos: Vec2<f32>, speed_factor: f32) -> impl Action<S> {
    now(move |ctx, _| {
        const WAYPOINT: f32 = 48.0;
        let start = ctx.npc.wpos.xy();
        let diff = wpos - start;
        let n = (diff.magnitude() / WAYPOINT).max(1.0);
        let mut points = (1..n as usize + 1).map(move |i| start + diff * (i as f32 / n));
        traverse_points(move |_| points.next(), speed_factor)
    })
    .debug(move || format!("travel to point {}, {}", wpos.x, wpos.y))
}

/// Try to travel to a site. Where practical, paths will be taken.
pub fn travel_to_site<S: State>(tgt_site: SiteId, speed_factor: f32) -> impl Action<S> {
    now(move |ctx, _| {
        let sites = &ctx.state.data().sites;

        let site_wpos = sites.get(tgt_site).map(|site| site.wpos.as_());

        // If we're currently in a site, try to find a path to the target site via
        // tracks
        if let Some(current_site) = ctx.npc.current_site
            && let Some(tracks) = path_between_towns(current_site, tgt_site, sites, ctx.world)
        {

            let mut path_nodes = tracks.path
                .into_iter()
                .flat_map(move |(track_id, reversed)| (0..)
                    .map(move |node_idx| (node_idx, track_id, reversed)));

            traverse_points(move |ctx| {
                let (node_idx, track_id, reversed) = path_nodes.next()?;
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
        } else if let Some(site) = sites.get(tgt_site) {
            // If all else fails, just walk toward the target site in a straight line
            travel_to_point(site.wpos.map(|e| e as f32 + 0.5), speed_factor).debug(|| "travel to point fallback").boxed()
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
