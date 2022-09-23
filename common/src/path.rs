use crate::{
    astar::{Astar, PathResult},
    terrain::Block,
    vol::{BaseVol, ReadVol},
};
use common_base::span;
use hashbrown::hash_map::DefaultHashBuilder;
#[cfg(rrt_pathfinding)] use hashbrown::HashMap;
#[cfg(rrt_pathfinding)]
use kiddo::{distance::squared_euclidean, KdTree}; // For RRT paths (disabled for now)
#[cfg(rrt_pathfinding)]
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
#[cfg(rrt_pathfinding)] use std::f32::consts::PI;
use std::iter::FromIterator;
use vek::*;

// Path

#[derive(Clone, Debug)]
pub struct Path<T> {
    nodes: Vec<T>,
}

impl<T> Default for Path<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::default(),
        }
    }
}

impl<T> FromIterator<T> for Path<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            nodes: iter.into_iter().collect(),
        }
    }
}

impl<T> IntoIterator for Path<T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter { self.nodes.into_iter() }
}

impl<T> Path<T> {
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

    pub fn len(&self) -> usize { self.nodes.len() }

    pub fn iter(&self) -> impl Iterator<Item = &T> { self.nodes.iter() }

    pub fn start(&self) -> Option<&T> { self.nodes.first() }

    pub fn end(&self) -> Option<&T> { self.nodes.last() }

    pub fn nodes(&self) -> &[T] { &self.nodes }
}

// Route: A path that can be progressed along

#[derive(Default, Clone, Debug)]
pub struct Route {
    path: Path<Vec3<i32>>,
    next_idx: usize,
}

impl From<Path<Vec3<i32>>> for Route {
    fn from(path: Path<Vec3<i32>>) -> Self { Self { path, next_idx: 0 } }
}

pub struct TraversalConfig {
    /// The distance to a node at which node is considered visited.
    pub node_tolerance: f32,
    /// The slowdown factor when following corners.
    /// 0.0 = no slowdown on corners, 1.0 = total slowdown on corners.
    pub slow_factor: f32,
    /// Whether the agent is currently on the ground.
    pub on_ground: bool,
    /// Whether the agent is currently in water.
    pub in_liquid: bool,
    /// The distance to the target below which it is considered reached.
    pub min_tgt_dist: f32,
    /// Whether the agent can climb.
    pub can_climb: bool,
    /// Whether the agent can fly.
    pub can_fly: bool,
}

const DIAGONALS: [Vec2<i32>; 8] = [
    Vec2::new(1, 0),
    Vec2::new(1, 1),
    Vec2::new(0, 1),
    Vec2::new(-1, 1),
    Vec2::new(-1, 0),
    Vec2::new(-1, -1),
    Vec2::new(0, -1),
    Vec2::new(1, -1),
];

impl Route {
    pub fn path(&self) -> &Path<Vec3<i32>> { &self.path }

    pub fn next(&self, i: usize) -> Option<Vec3<i32>> {
        self.path.nodes.get(self.next_idx + i).copied()
    }

    pub fn is_finished(&self) -> bool { self.next(0).is_none() }

    pub fn traverse<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        traversal_cfg: &TraversalConfig,
    ) -> Option<(Vec3<f32>, f32)>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let (next0, next1, next_tgt, be_precise) = loop {
            // If we've reached the end of the path, stop
            let next0 = self.next(0)?;
            let next1 = self.next(1).unwrap_or(next0);

            // Stop using obstructed paths
            if !walkable(vol, next1) {
                return None;
            }

            let be_precise = DIAGONALS.iter().any(|pos| {
                (-1..2).all(|z| {
                    vol.get(next0 + Vec3::new(pos.x, pos.y, z))
                        .map(|b| !b.is_solid())
                        .unwrap_or(false)
                })
            });

            // Map position of node to middle of block
            let next_tgt = next0.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0);
            let closest_tgt = next_tgt.map2(pos, |tgt, pos| pos.clamped(tgt.floor(), tgt.ceil()));
            // Determine whether we're close enough to the next to to consider it completed
            let dist_sqrd = pos.xy().distance_squared(closest_tgt.xy());
            if dist_sqrd
                < traversal_cfg.node_tolerance.powi(2) * if be_precise { 0.25 } else { 1.0 }
                && (((pos.z - closest_tgt.z > 1.2 || (pos.z - closest_tgt.z > -0.2 && traversal_cfg.on_ground))
                    && (pos.z - closest_tgt.z < 1.2 || (pos.z - closest_tgt.z < 2.9 && vel.z < -0.05))
                    && vel.z <= 0.0
                    // Only consider the node reached if there's nothing solid between us and it
                    && (vol
                        .ray(pos + Vec3::unit_z() * 1.5, closest_tgt + Vec3::unit_z() * 1.5)
                        .until(Block::is_solid)
                        .cast()
                        .0
                        > pos.distance(closest_tgt) * 0.9 || dist_sqrd < 0.5)
                    && self.next_idx < self.path.len())
                    || (traversal_cfg.in_liquid
                        && pos.z < closest_tgt.z + 0.8
                        && pos.z > closest_tgt.z))
            {
                // Node completed, move on to the next one
                self.next_idx += 1;
            } else {
                // The next node hasn't been reached yet, use it as a target
                break (next0, next1, next_tgt, be_precise);
            }
        };

        fn gradient(line: LineSegment2<f32>) -> f32 {
            let r = (line.start.y - line.end.y) / (line.start.x - line.end.x);
            if r.is_nan() { 100000.0 } else { r }
        }

        fn intersect(a: LineSegment2<f32>, b: LineSegment2<f32>) -> Option<Vec2<f32>> {
            let ma = gradient(a);
            let mb = gradient(b);

            let ca = a.start.y - ma * a.start.x;
            let cb = b.start.y - mb * b.start.x;

            if (ma - mb).abs() < 0.0001 || (ca - cb).abs() < 0.0001 {
                None
            } else {
                let x = (cb - ca) / (ma - mb);
                let y = ma * x + ca;

                Some(Vec2::new(x, y))
            }
        }

        // We don't always want to aim for the centre of block since this can create
        // jerky zig-zag movement. This function attempts to find a position
        // inside a target block's area that aligned nicely with our velocity.
        // This has a twofold benefit:
        //
        // 1. Entities can move at any angle when
        // running on a flat surface
        //
        // 2. We don't have to search diagonals when
        // pathfinding - cartesian positions are enough since this code will
        // make the entity move smoothly along them
        let corners = [
            Vec2::new(0, 0),
            Vec2::new(1, 0),
            Vec2::new(1, 1),
            Vec2::new(0, 1),
            Vec2::new(0, 0), // Repeated start
        ];

        let vel_line = LineSegment2 {
            start: pos.xy(),
            end: pos.xy() + vel.xy() * 100.0,
        };

        let align = |block_pos: Vec3<i32>, precision: f32| {
            let lerp_block =
                |x, precision| Lerp::lerp(x, block_pos.xy().map(|e| e as f32), precision);

            (0..4)
                .filter_map(|i| {
                    let edge_line = LineSegment2 {
                        start: lerp_block(
                            (block_pos.xy() + corners[i]).map(|e| e as f32),
                            precision,
                        ),
                        end: lerp_block(
                            (block_pos.xy() + corners[i + 1]).map(|e| e as f32),
                            precision,
                        ),
                    };
                    intersect(vel_line, edge_line).filter(|intersect| {
                        intersect
                            .clamped(
                                block_pos.xy().map(|e| e as f32),
                                block_pos.xy().map(|e| e as f32 + 1.0),
                            )
                            .distance_squared(*intersect)
                            < 0.001
                    })
                })
                .min_by_key(|intersect: &Vec2<f32>| {
                    (intersect.distance_squared(vel_line.end) * 1000.0) as i32
                })
                .unwrap_or_else(|| {
                    (0..2)
                        .flat_map(|i| (0..2).map(move |j| Vec2::new(i, j)))
                        .map(|rpos| block_pos + rpos)
                        .map(|block_pos| {
                            let block_posf = block_pos.xy().map(|e| e as f32);
                            let proj = vel_line.projected_point(block_posf);
                            let clamped = lerp_block(
                                proj.clamped(
                                    block_pos.xy().map(|e| e as f32),
                                    block_pos.xy().map(|e| e as f32),
                                ),
                                precision,
                            );

                            (proj.distance_squared(clamped), clamped)
                        })
                        .min_by_key(|(d2, _)| (d2 * 1000.0) as i32)
                        .unwrap()
                        .1
                })
        };

        let bez = CubicBezier2 {
            start: pos.xy(),
            ctrl0: pos.xy() + vel.xy().try_normalized().unwrap_or_default() * 1.0,
            ctrl1: align(next0, 1.0),
            end: align(next1, 1.0),
        };

        // Use a cubic spline of the next few targets to come up with a sensible target
        // position. We want to use a position that gives smooth movement but is
        // also accurate enough to avoid the agent getting stuck under ledges or
        // falling off walls.
        let next_dir = bez
            .evaluate_derivative(0.85)
            .try_normalized()
            .unwrap_or_default();
        let straight_factor = next_dir
            .dot(vel.xy().try_normalized().unwrap_or(next_dir))
            .max(0.0)
            .powi(2);

        let bez = CubicBezier2 {
            start: pos.xy(),
            ctrl0: pos.xy() + vel.xy().try_normalized().unwrap_or_default() * 1.0,
            ctrl1: align(
                next0,
                (1.0 - if (next0.z as f32 - pos.z).abs() < 0.25 && !be_precise {
                    straight_factor
                } else {
                    0.0
                })
                .max(0.1),
            ),
            end: align(next1, 1.0),
        };

        let tgt2d = bez.evaluate(if (next0.z as f32 - pos.z).abs() < 0.25 {
            0.25
        } else {
            0.5
        });
        let tgt = if be_precise {
            next_tgt
        } else {
            Vec3::from(tgt2d) + Vec3::unit_z() * next_tgt.z
        };

        Some((
            tgt - pos,
            // Control the entity's speed to hopefully stop us falling off walls on sharp
            // corners. This code is very imperfect: it does its best but it
            // can still fail for particularly fast entities.
            straight_factor * traversal_cfg.slow_factor + (1.0 - traversal_cfg.slow_factor),
        ))
        .filter(|(bearing, _)| bearing.z < 2.1)
    }
}

/// A self-contained system that attempts to chase a moving target, only
/// performing pathfinding if necessary
#[derive(Default, Clone, Debug)]
pub struct Chaser {
    last_search_tgt: Option<Vec3<f32>>,
    /// `bool` indicates whether the Route is a complete route to the target
    route: Option<(Route, bool)>,
    /// We use this hasher (AAHasher) because:
    /// (1) we care about DDOS attacks (ruling out FxHash);
    /// (2) we don't care about determinism across computers (we can use
    /// AAHash).
    astar: Option<Astar<Vec3<i32>, DefaultHashBuilder>>,
}

impl Chaser {
    /// Returns bearing and speed
    /// Bearing is a Vec3<f32> dictating the direction of movement
    /// Speed is an f32 between 0.0 and 1.0
    pub fn chase<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        tgt: Vec3<f32>,
        traversal_cfg: TraversalConfig,
    ) -> Option<(Vec3<f32>, f32)>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        span!(_guard, "chase", "Chaser::chase");
        let pos_to_tgt = pos.distance(tgt);

        // If we're already close to the target then there's nothing to do
        let end = self
            .route
            .as_ref()
            .and_then(|(r, _)| r.path.end().copied())
            .map(|e| e.map(|e| e as f32 + 0.5))
            .unwrap_or(tgt);
        if ((pos - end) * Vec3::new(1.0, 1.0, 2.0)).magnitude_squared()
            < traversal_cfg.min_tgt_dist.powi(2)
        {
            self.route = None;
            return None;
        }

        let bearing = if let Some((end, complete)) = self
            .route
            .as_ref()
            .and_then(|(r, complete)| Some((r.path().end().copied()?, *complete)))
        {
            let end_to_tgt = end.map(|e| e as f32).distance(tgt);
            // If the target has moved significantly since the path was generated then it's
            // time to search for a new path. Also, do this randomly from time
            // to time to avoid any edge cases that cause us to get stuck. In
            // theory this shouldn't happen, but in practice the world is full
            // of unpredictable obstacles that are more than willing to mess up
            // our day. TODO: Come up with a better heuristic for this
            if end_to_tgt > pos_to_tgt * 0.3 + 5.0 && complete {
                None
            } else if thread_rng().gen::<f32>() < 0.001 {
                self.route = None;
                None
            } else {
                self.route
                    .as_mut()
                    .and_then(|(r, _)| r.traverse(vol, pos, vel, &traversal_cfg))
            }
        } else {
            // There is no route found yet
            None
        };

        // If a bearing has already been determined, use that
        if let Some((bearing, speed)) = bearing {
            Some((bearing, speed))
        } else {
            // Since no bearing has been determined yet, a new route will be
            // calculated if the target has moved, pathfinding is not complete,
            // or there is no route
            let tgt_dir = (tgt - pos).xy().try_normalized().unwrap_or_default();

            // Only search for a path if the target has moved from their last position. We
            // don't want to be thrashing the pathfinding code for targets that
            // we're unable to access!
            if self
                .last_search_tgt
                .map(|last_tgt| last_tgt.distance(tgt) > pos_to_tgt * 0.15 + 5.0)
                .unwrap_or(true)
                || self.astar.is_some()
                || self.route.is_none()
            {
                self.last_search_tgt = Some(tgt);

                // NOTE: Enable air paths when air braking has been figured out
                let (path, complete) = /*if cfg!(rrt_pathfinding) && traversal_cfg.can_fly {
                    find_air_path(vol, pos, tgt, &traversal_cfg)
                } else */{
                    find_path(&mut self.astar, vol, pos, tgt, &traversal_cfg)
                };

                self.route = path.map(|path| {
                    let start_index = path
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, node)| {
                            node.map(|e| e as f32).distance_squared(pos + tgt_dir) as i32
                        })
                        .map(|(idx, _)| idx);

                    (
                        Route {
                            path,
                            next_idx: start_index.unwrap_or(0),
                        },
                        complete,
                    )
                });
            }
            // Start traversing the new route if it exists
            if let Some(bearing) = self
                .route
                .as_mut()
                .and_then(|(r, _)| r.traverse(vol, pos, vel, &traversal_cfg))
            {
                Some(bearing)
            } else {
                // At this point no route is available and no bearing
                // has been determined, so we start sampling terrain.
                // Check for falling off walls and try moving straight
                // towards the target if falling is not a danger
                let walking_towards_edge = (-3..2).all(|z| {
                    vol.get(
                        (pos + Vec3::<f32>::from(tgt_dir) * 2.5).map(|e| e as i32)
                            + Vec3::unit_z() * z,
                    )
                    .map(|b| b.is_air())
                    .unwrap_or(false)
                });

                // Enable when airbraking/flight is figured out
                /*if traversal_cfg.can_fly {
                    Some(((tgt - pos) , 1.0))
                } else */
                if !walking_towards_edge || traversal_cfg.can_fly {
                    Some(((tgt - pos) * Vec3::new(1.0, 1.0, 0.0), 1.0))
                } else {
                    // This is unfortunately where an NPC will stare blankly
                    // into space. No route has been found and no temporary
                    // bearing would suffice. Hopefully a route will be found
                    // in the coming ticks.
                    None
                }
            }
        }
    }
}

fn walkable<V>(vol: &V, pos: Vec3<i32>) -> bool
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let below = vol
        .get(pos - Vec3::unit_z())
        .ok()
        .copied()
        .unwrap_or_else(Block::empty);
    let a = vol.get(pos).ok().copied().unwrap_or_else(Block::empty);
    let b = vol
        .get(pos + Vec3::unit_z())
        .ok()
        .copied()
        .unwrap_or_else(Block::empty);

    let on_ground = below.is_filled();
    let in_liquid = a.is_liquid();
    (on_ground || in_liquid) && !a.is_solid() && !b.is_solid()
}

/// Attempt to search for a path to a target, returning the path (if one was
/// found) and whether it is complete (reaches the target)
fn find_path<V>(
    astar: &mut Option<Astar<Vec3<i32>, DefaultHashBuilder>>,
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    traversal_cfg: &TraversalConfig,
) -> (Option<Path<Vec3<i32>>>, bool)
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let is_walkable = |pos: &Vec3<i32>| walkable(vol, *pos);
    let get_walkable_z = |pos| {
        let mut z_incr = 0;
        for _ in 0..32 {
            let test_pos = pos + Vec3::unit_z() * z_incr;
            if is_walkable(&test_pos) {
                return Some(test_pos);
            }
            z_incr = -z_incr + i32::from(z_incr <= 0);
        }
        None
    };

    let (start, end) = match (
        get_walkable_z(startf.map(|e| e.floor() as i32)),
        get_walkable_z(endf.map(|e| e.floor() as i32)),
    ) {
        (Some(start), Some(end)) => (start, end),
        _ => return (None, false),
    };

    let heuristic = |pos: &Vec3<i32>| (pos.distance_squared(end) as f32).sqrt();
    let neighbors = |pos: &Vec3<i32>| {
        let pos = *pos;
        const DIRS: [Vec3<i32>; 17] = [
            Vec3::new(0, 1, 0),   // Forward
            Vec3::new(0, 1, 1),   // Forward upward
            Vec3::new(0, 1, -1),  // Forward downward
            Vec3::new(0, 1, -2),  // Forward downwardx2
            Vec3::new(1, 0, 0),   // Right
            Vec3::new(1, 0, 1),   // Right upward
            Vec3::new(1, 0, -1),  // Right downward
            Vec3::new(1, 0, -2),  // Right downwardx2
            Vec3::new(0, -1, 0),  // Backwards
            Vec3::new(0, -1, 1),  // Backward Upward
            Vec3::new(0, -1, -1), // Backward downward
            Vec3::new(0, -1, -2), // Backward downwardx2
            Vec3::new(-1, 0, 0),  // Left
            Vec3::new(-1, 0, 1),  // Left upward
            Vec3::new(-1, 0, -1), // Left downward
            Vec3::new(-1, 0, -2), // Left downwardx2
            Vec3::new(0, 0, -1),  // Downwards
        ];

        const JUMPS: [Vec3<i32>; 4] = [
            Vec3::new(0, 1, 2),  // Forward Upwardx2
            Vec3::new(1, 0, 2),  // Right Upwardx2
            Vec3::new(0, -1, 2), // Backward Upwardx2
            Vec3::new(-1, 0, 2), // Left Upwardx2
        ];

        // let walkable = [
        //     is_walkable(&(pos + Vec3::new(1, 0, 0))),
        //     is_walkable(&(pos + Vec3::new(-1, 0, 0))),
        //     is_walkable(&(pos + Vec3::new(0, 1, 0))),
        //     is_walkable(&(pos + Vec3::new(0, -1, 0))),
        // ];

        // const DIAGONALS: [(Vec3<i32>, [usize; 2]); 8] = [
        //     (Vec3::new(1, 1, 0), [0, 2]),
        //     (Vec3::new(-1, 1, 0), [1, 2]),
        //     (Vec3::new(1, -1, 0), [0, 3]),
        //     (Vec3::new(-1, -1, 0), [1, 3]),
        //     (Vec3::new(1, 1, 1), [0, 2]),
        //     (Vec3::new(-1, 1, 1), [1, 2]),
        //     (Vec3::new(1, -1, 1), [0, 3]),
        //     (Vec3::new(-1, -1, 1), [1, 3]),
        // ];

        DIRS.iter()
            .chain(
                Some(JUMPS.iter())
                    .filter(|_| {
                        vol.get(pos - Vec3::unit_z())
                            .map(|b| !b.is_liquid())
                            .unwrap_or(true)
                            || traversal_cfg.can_climb
                            || traversal_cfg.can_fly
                    })
                    .into_iter()
                    .flatten(),
            )
            .map(move |dir| (pos, dir))
            .filter(move |(pos, dir)| {
                (traversal_cfg.can_fly || is_walkable(pos) && is_walkable(&(*pos + **dir)))
                    && ((dir.z < 1
                        || vol
                            .get(pos + Vec3::unit_z() * 2)
                            .map(|b| !b.is_solid())
                            .unwrap_or(true))
                        && (dir.z < 2
                            || vol
                                .get(pos + Vec3::unit_z() * 3)
                                .map(|b| !b.is_solid())
                                .unwrap_or(true))
                        && (dir.z >= 0
                            || vol
                                .get(pos + *dir + Vec3::unit_z() * 2)
                                .map(|b| !b.is_solid())
                                .unwrap_or(true)))
            })
            .map(move |(pos, dir)| pos + dir)
        // .chain(
        //     DIAGONALS
        //         .iter()
        //         .filter(move |(dir, [a, b])| {
        //             is_walkable(&(pos + *dir)) && walkable[*a] &&
        // walkable[*b]         })
        //         .map(move |(dir, _)| pos + *dir),
        // )
    };

    let transition = |a: &Vec3<i32>, b: &Vec3<i32>| {
        let crow_line = LineSegment2 {
            start: startf.xy(),
            end: endf.xy(),
        };

        // Modify the heuristic a little in order to prefer paths that take us on a
        // straight line toward our target. This means we get smoother movement.
        1.0 + crow_line.distance_to_point(b.xy().map(|e| e as f32)) * 0.025
            + (b.z - a.z - 1).max(0) as f32 * 10.0
    };
    let satisfied = |pos: &Vec3<i32>| pos == &end;

    let mut new_astar = match astar.take() {
        None => Astar::new(25_000, start, heuristic, DefaultHashBuilder::default()),
        Some(astar) => astar,
    };

    let path_result = new_astar.poll(100, heuristic, neighbors, transition, satisfied);

    *astar = Some(new_astar);

    match path_result {
        PathResult::Path(path) => {
            *astar = None;
            (Some(path), true)
        },
        PathResult::None(path) => {
            *astar = None;
            (Some(path), false)
        },
        PathResult::Exhausted(path) => {
            *astar = None;
            (Some(path), false)
        },
        PathResult::Pending => (None, false),
    }
}

// Enable when airbraking/sensible flight is a thing
#[cfg(rrt_pathfinding)]
fn find_air_path<V>(
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    traversal_cfg: &TraversalConfig,
) -> (Option<Path<Vec3<i32>>>, bool)
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let radius = traversal_cfg.node_tolerance;
    let mut connect = false;
    let total_dist_sqrd = startf.distance_squared(endf);
    // First check if a straight line path works
    if vol
        .ray(startf + Vec3::unit_z(), endf + Vec3::unit_z())
        .until(Block::is_opaque)
        .cast()
        .0
        .powi(2)
        >= total_dist_sqrd
    {
        let mut path = Vec::new();
        path.push(endf.map(|e| e.floor() as i32));
        connect = true;
        (Some(path.into_iter().collect()), connect)
    // Else use RRTs
    } else {
        let is_traversable = |start: &Vec3<f32>, end: &Vec3<f32>| {
            vol.ray(*start, *end)
                .until(Block::is_solid)
                .cast()
                .0
                .powi(2)
                > (*start).distance_squared(*end)
            //vol.get(*pos).ok().copied().unwrap_or_else(Block::empty).
            // is_fluid();
        };
        informed_rrt_connect(start, end, is_traversable)
    }
}

/// Attempts to find a path from a start to the end using an informed
/// RRT-Connect algorithm. A point is sampled from a bounding spheroid
/// between the start and end. Two separate rapidly exploring random
/// trees extend toward the sampled point. Nodes are stored in k-d trees
/// for quicker nearest node calculations. Points are sampled until the
/// trees connect. A final path is then reconstructed from the nodes.
/// This pathfinding algorithm is more appropriate for 3D pathfinding
/// with wider gaps, such as flying through a forest than for terrain
/// with narrow gaps, such as navigating a maze.
/// Returns a path and whether that path is complete or not.
#[cfg(rrt_pathfinding)]
fn informed_rrt_connect(
    start: Vec3<f32>,
    end: Vec3<f32>,
    is_valid_edge: impl Fn(&Vec3<f32>, &Vec3<f32>) -> bool,
) -> (Option<Path<Vec3<i32>>>, bool) {
    let mut path = Vec::new();

    // Each tree has a vector of nodes
    let mut node_index1: usize = 0;
    let mut node_index2: usize = 0;
    let mut nodes1 = Vec::new();
    let mut nodes2 = Vec::new();

    // The parents hashmap stores nodes and their parent nodes as pairs to
    // retrace the complete path once the two RRTs connect
    let mut parents1 = HashMap::new();
    let mut parents2 = HashMap::new();

    // The path vector stores the path from the appropriate terminal to the
    // connecting node or vice versa
    let mut path1 = Vec::new();
    let mut path2 = Vec::new();

    // K-d trees are used to find the closest nodes rapidly
    let mut kdtree1 = KdTree::new();
    let mut kdtree2 = KdTree::new();

    // Add the start as the first node of the first k-d tree
    kdtree1
        .add(&[startf.x, startf.y, startf.z], node_index1)
        .unwrap_or_default();
    nodes1.push(startf);
    node_index1 += 1;

    // Add the end as the first node of the second k-d tree
    kdtree2
        .add(&[endf.x, endf.y, endf.z], node_index2)
        .unwrap_or_default();
    nodes2.push(endf);
    node_index2 += 1;

    let mut connection1_idx = 0;
    let mut connection2_idx = 0;

    let mut connect = false;

    // Scalar non-dimensional value that is proportional to the size of the
    // sample spheroid volume. This increases in value until a path is found.
    let mut search_parameter = 0.01;

    // Maximum of 7000 iterations
    for _i in 0..7000 {
        if connect {
            break;
        }

        // Sample a point on the bounding spheroid
        let (sampled_point1, sampled_point2) = {
            let point = point_on_prolate_spheroid(startf, endf, search_parameter);
            (point, point)
        };

        // Find the nearest nodes to the the sampled point
        let nearest_index1 = kdtree1
            .nearest_one(
                &[sampled_point1.x, sampled_point1.y, sampled_point1.z],
                &squared_euclidean,
            )
            .map_or(0, |n| *n.1);
        let nearest_index2 = kdtree2
            .nearest_one(
                &[sampled_point2.x, sampled_point2.y, sampled_point2.z],
                &squared_euclidean,
            )
            .map_or(0, |n| *n.1);
        let nearest1 = nodes1[nearest_index1];
        let nearest2 = nodes2[nearest_index2];

        // Extend toward the sampled point from the nearest node of each tree
        let new_point1 = nearest1 + (sampled_point1 - nearest1).normalized().map(|a| a * radius);
        let new_point2 = nearest2 + (sampled_point2 - nearest2).normalized().map(|a| a * radius);

        // Ensure the new nodes are valid/traversable
        if is_valid_edge(&nearest1, &new_point1) {
            kdtree1
                .add(&[new_point1.x, new_point1.y, new_point1.z], node_index1)
                .unwrap_or_default();
            nodes1.push(new_point1);
            parents1.insert(node_index1, nearest_index1);
            node_index1 += 1;
            // Check if the trees connect
            if let Ok((check, index)) = kdtree2.nearest_one(
                &[new_point1.x, new_point1.y, new_point1.z],
                &squared_euclidean,
            ) {
                if check < radius {
                    let connection = nodes2[*index];
                    connection2_idx = *index;
                    nodes1.push(connection);
                    connection1_idx = nodes1.len() - 1;
                    parents1.insert(node_index1, node_index1 - 1);
                    connect = true;
                }
            }
        }

        // Repeat the validity check for the second tree
        if is_valid_edge(&nearest2, &new_point2) {
            kdtree2
                .add(&[new_point2.x, new_point2.y, new_point1.z], node_index2)
                .unwrap_or_default();
            nodes2.push(new_point2);
            parents2.insert(node_index2, nearest_index2);
            node_index2 += 1;
            // Again check for a connection
            if let Ok((check, index)) = kdtree1.nearest_one(
                &[new_point2.x, new_point2.y, new_point1.z],
                &squared_euclidean,
            ) {
                if check < radius {
                    let connection = nodes1[*index];
                    connection1_idx = *index;
                    nodes2.push(connection);
                    connection2_idx = nodes2.len() - 1;
                    parents2.insert(node_index2, node_index2 - 1);
                    connect = true;
                }
            }
        }
        // Increase the search parameter to widen the sample volume
        search_parameter += 0.02;
    }

    if connect {
        // Construct paths from the connection node to the start and end
        let mut current_node_index1 = connection1_idx;
        while current_node_index1 > 0 {
            current_node_index1 = *parents1.get(&current_node_index1).unwrap_or(&0);
            path1.push(nodes1[current_node_index1].map(|e| e.floor() as i32));
        }
        let mut current_node_index2 = connection2_idx;
        while current_node_index2 > 0 {
            current_node_index2 = *parents2.get(&current_node_index2).unwrap_or(&0);
            path2.push(nodes2[current_node_index2].map(|e| e.floor() as i32));
        }
        // Join the two paths together in the proper order and remove duplicates
        path1.pop();
        path1.reverse();
        path.append(&mut path1);
        path.append(&mut path2);
        path.dedup();
    } else {
        // If the trees did not connect, construct a path from the start to
        // the closest node to the end
        let mut current_node_index1 = kdtree1
            .nearest_one(&[endf.x, endf.y, endf.z], &squared_euclidean)
            .map_or(0, |c| *c.1);
        // Attempt to pick a node other than the start node
        for _i in 0..3 {
            if current_node_index1 == 0
                || nodes1[current_node_index1].distance_squared(startf) < 4.0
            {
                if let Some(index) = parents1.values().choose(&mut thread_rng()) {
                    current_node_index1 = *index;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        path1.push(nodes1[current_node_index1].map(|e| e.floor() as i32));
        // Construct the path
        while current_node_index1 != 0 && nodes1[current_node_index1].distance_squared(startf) > 4.0
        {
            current_node_index1 = *parents1.get(&current_node_index1).unwrap_or(&0);
            path1.push(nodes1[current_node_index1].map(|e| e.floor() as i32));
        }

        path1.reverse();
        path.append(&mut path1);
    }
    let mut new_path = Vec::new();
    let mut node = path[0];
    new_path.push(node);
    let mut node_idx = 0;
    let num_nodes = path.len();
    let end = path[num_nodes - 1];
    while node != end {
        let next_idx = if node_idx + 4 > num_nodes - 1 {
            num_nodes - 1
        } else {
            node_idx + 4
        };
        let next_node = path[next_idx];
        let start_pos = node.map(|e| e as f32 + 0.5);
        let end_pos = next_node.map(|e| e as f32 + 0.5);
        if vol
            .ray(start_pos, end_pos)
            .until(Block::is_solid)
            .cast()
            .0
            .powi(2)
            > (start_pos).distance_squared(end_pos)
        {
            node_idx = next_idx;
            new_path.push(next_node);
        } else {
            node_idx += 1;
        }
        node = path[node_idx];
    }
    path = new_path;
}

/// Returns a random point within a radially symmetrical ellipsoid with given
/// foci and a `search parameter` to determine the size of the ellipse beyond
/// the foci. Technically the point is within a prolate spheroid translated and
/// rotated to the proper place in cartesian space.
/// The search_parameter is a float that relates to the length of the string for
/// a two dimensional ellipse or the size of the ellipse beyond the foci. In
/// this case that analogy still holds as the ellipse is radially symmetrical
/// along the axis between the foci. The value of the search parameter must be
/// greater than zero. In order to increase the sample area, the
/// search_parameter should be increased linearly as the search continues.
#[cfg(rrt_pathfinding)]
pub fn point_on_prolate_spheroid(
    focus1: Vec3<f32>,
    focus2: Vec3<f32>,
    search_parameter: f32,
) -> Vec3<f32> {
    let mut rng = thread_rng();
    // Uniform distribution
    let range = Uniform::from(0.0..1.0);

    // Midpoint is used as the local origin
    let midpoint = 0.5 * (focus1 + focus2);
    // Radius between the start and end of the path
    let radius: f32 = focus1.distance(focus2);
    // The linear eccentricity of an ellipse is the distance from the origin to a
    // focus A prolate spheroid is a half-ellipse rotated for a full revolution
    // which is why ellipse variables are used frequently in this function
    let linear_eccentricity: f32 = 0.5 * radius;

    // For an ellipsoid, three variables determine the shape: a, b, and c.
    // These are the distance from the center/origin to the surface on the
    // x, y, and z axes, respectively.
    // For a prolate spheroid a and b are equal.
    // c is determined by adding the search parameter to the linear eccentricity.
    // As the search parameter increases the size of the spheroid increases
    let c: f32 = linear_eccentricity + search_parameter;
    // The width is calculated to prioritize increasing width over length of
    // the ellipsoid
    let a: f32 = (c.powi(2) - linear_eccentricity.powi(2)).powf(0.5);
    // The width should be the same in both the x and y directions
    let b: f32 = a;

    // The parametric spherical equation for an ellipsoid measuring from the
    // center point is as follows:
    // x = a * cos(theta) * cos(lambda)
    // y = b * cos(theta) * sin(lambda)
    // z = c * sin(theta)
    //
    // where     -0.5 * PI <= theta <= 0.5 * PI
    // and       0.0 <= lambda < 2.0 * PI
    //
    // Select these two angles using the uniform distribution defined at the
    // beginning of the function from 0.0 to 1.0
    let rtheta: f32 = PI * range.sample(&mut rng) - 0.5 * PI;
    let lambda: f32 = 2.0 * PI * range.sample(&mut rng);
    // Select a point on the surface of the ellipsoid
    let point = Vec3::new(
        a * rtheta.cos() * lambda.cos(),
        b * rtheta.cos() * lambda.sin(),
        c * rtheta.sin(),
    );
    // NOTE: Theoretically we should sample a point within the spheroid
    // requiring selecting a point along the radius. In my tests selecting
    // a point *on the surface* of the spheroid results in sampling that is
    // "good enough". The following code is commented out to reduce expense.
    //let surface_point = Vec3::new(a * rtheta.cos() * lambda.cos(), b *
    // rtheta.cos() * lambda.sin(), c * rtheta.sin()); let magnitude =
    // surface_point.magnitude(); let direction = surface_point.normalized();
    //// Randomly select a point along the vector to the previously selected surface
    //// point using the uniform distribution
    //let point = magnitude * range.sample(&mut rng) * direction;

    // Now that a point has been selected in local space, it must be rotated and
    // translated into global coordinates
    // NOTE: Don't rotate about the z axis as the point is already randomly
    // selected about the z axis
    //let dx = focus2.x - focus1.x;
    //let dy = focus2.y - focus1.y;
    let dz = focus2.z - focus1.z;
    // Phi and theta are the angles from the x axis in the x-y plane and from
    // the z axis, respectively. (As found in spherical coordinates)
    // These angles are used to rotate the random point in the spheroid about
    // the local origin
    //
    // Rotate about z axis by phi
    //let phi: f32 = if dx.abs() > 0.0 {
    //    (dy / dx).atan()
    //} else {
    //    0.5 * PI
    //};
    // This is unnecessary as rtheta is randomly selected between 0.0 and 2.0 * PI
    // let rot_z_mat = Mat3::new(phi.cos(), -1.0 * phi.sin(), 0.0, phi.sin(),
    // phi.cos(), 0.0, 0.0, 0.0, 1.0);

    // Rotate about perpendicular vector in the xy plane by theta
    let theta: f32 = if radius > 0.0 {
        (dz / radius).acos()
    } else {
        0.0
    };
    // Vector from focus1 to focus2
    let r_vec = focus2 - focus1;
    // Perpendicular vector in xy plane
    let perp_vec = Vec3::new(-1.0 * r_vec.y, r_vec.x, 0.0).normalized();
    let l = perp_vec.x;
    let m = perp_vec.y;
    let n = perp_vec.z;
    // Rotation matrix for rotation about a vector
    let rot_2_mat = Mat3::new(
        l * l * (1.0 - theta.cos()),
        m * l * (1.0 - theta.cos()) - n * theta.sin(),
        n * l * (1.0 - theta.cos()) + m * theta.sin(),
        l * m * (1.0 - theta.cos()) + n * theta.sin(),
        m * m * (1.0 - theta.cos()) + theta.cos(),
        n * m * (1.0 - theta.cos()) - l * theta.sin(),
        l * n * (1.0 - theta.cos()) - m * theta.sin(),
        m * n * (1.0 - theta.cos()) + l * theta.sin(),
        n * n * (1.0 - theta.cos()) + theta.cos(),
    );

    // Get the global coordinates of the point by rotating and adding the origin
    // rot_z_mat is unneeded due to the random rotation defined by lambda
    // let global_coords = midpoint + rot_2_mat * (rot_z_mat * point);
    midpoint + rot_2_mat * point
}
