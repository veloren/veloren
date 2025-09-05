use crate::{
    astar::{Astar, PathResult},
    resources::Time,
    terrain::Block,
    vol::{BaseVol, ReadVol},
};
use common_base::span;
use fxhash::FxBuildHasher;
#[cfg(feature = "rrt_pathfinding")]
use hashbrown::HashMap;
#[cfg(feature = "rrt_pathfinding")]
use kiddo::{SquaredEuclidean, float::kdtree::KdTree, nearest_neighbour::NearestNeighbour}; /* For RRT paths (disabled for now) */
use rand::{Rng, rng};
#[cfg(feature = "rrt_pathfinding")]
use rand::{
    distributions::{Distribution, Uniform},
    prelude::IteratorRandom,
};
#[cfg(feature = "rrt_pathfinding")]
use std::f32::consts::PI;
use std::{collections::VecDeque, iter::FromIterator};
use vek::*;

// Path

#[derive(Clone, Debug)]
pub struct Path<T> {
    pub nodes: Vec<T>,
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

impl Route {
    pub fn get_path(&self) -> &Path<Vec3<i32>> { &self.path }

    pub fn next_idx(&self) -> usize { self.next_idx }
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
    /// Whether the agent has vectored propulsion.
    pub vectored_propulsion: bool,
    /// Whether chunk containing target position is currently loaded
    pub is_target_loaded: bool,
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

pub enum TraverseStop {
    Done,
    InvalidOutput,
    InvalidPath,
}

impl Route {
    pub fn path(&self) -> &Path<Vec3<i32>> { &self.path }

    pub fn next(&self, i: usize) -> Option<Vec3<i32>> {
        self.path.nodes.get(self.next_idx + i).copied()
    }

    pub fn is_finished(&self) -> bool { self.next(0).is_none() }

    /// Handles moving along a path.
    pub fn traverse<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        traversal_cfg: &TraversalConfig,
    ) -> Result<(Vec3<f32>, f32), TraverseStop>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let (next0, next1, next_tgt, be_precise) = loop {
            // If we've reached the end of the path, stop
            let next0 = self.next(0).ok_or(TraverseStop::Done)?;
            let next1 = self.next(1).unwrap_or(next0);

            // Stop using obstructed paths
            if !walkable(vol, next0, traversal_cfg.is_target_loaded)
                || !walkable(vol, next1, traversal_cfg.is_target_loaded)
            {
                return Err(TraverseStop::InvalidPath);
            }

            // If, in any direction, there is a column of open air of several blocks
            let open_space_nearby = DIAGONALS.iter().any(|pos| {
                (-2..2).all(|z| {
                    vol.get(next0 + Vec3::new(pos.x, pos.y, z))
                        .map(|b| !b.is_solid())
                        .unwrap_or(false)
                })
            });

            // If, in any direction, there is a solid wall
            let wall_nearby = DIAGONALS.iter().any(|pos| {
                vol.get(next0 + Vec3::new(pos.x, pos.y, 1))
                    .map(|b| b.is_solid())
                    .unwrap_or(true)
            });

            // Unwalkable obstacles, such as walls or open space or stepping up blocks can
            // affect path-finding
            let be_precise =
                open_space_nearby || wall_nearby || (pos.z - next0.z as f32).abs() > 1.0;

            // If we're not being precise and the next next target is closer, go towards
            // that instead.
            if !be_precise
                && next0.as_::<f32>().distance_squared(pos)
                    > next1.as_::<f32>().distance_squared(pos)
            {
                self.next_idx += 1;
                continue;
            }

            // Map position of node to middle of block
            let next_tgt = next0.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0);
            let closest_tgt = next_tgt
                .map2(pos, |tgt, pos| pos.clamped(tgt.floor(), tgt.ceil()))
                .xy()
                .with_z(next_tgt.z);
            // Determine whether we're close enough to the next to to consider it completed
            let dist_sqrd = pos.xy().distance_squared(closest_tgt.xy());
            if dist_sqrd
                < (traversal_cfg.node_tolerance
                    * if be_precise {
                        0.5
                    } else if traversal_cfg.in_liquid {
                        2.5
                    } else {
                        1.0
                    })
                .powi(2)
                && ((-1.0..=2.25).contains(&(pos.z - closest_tgt.z))
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

        let line_segments = [
            LineSegment3 {
                start: self
                    .next_idx
                    .checked_sub(2)
                    .and_then(|i| self.path().nodes().get(i))
                    .unwrap_or(&next0)
                    .as_()
                    + 0.5,
                end: self
                    .next_idx
                    .checked_sub(1)
                    .and_then(|i| self.path().nodes().get(i))
                    .unwrap_or(&next0)
                    .as_()
                    + 0.5,
            },
            LineSegment3 {
                start: self
                    .next_idx
                    .checked_sub(1)
                    .and_then(|i| self.path().nodes().get(i))
                    .unwrap_or(&next0)
                    .as_()
                    + 0.5,
                end: next0.as_() + 0.5,
            },
            LineSegment3 {
                start: next0.as_() + 0.5,
                end: next1.as_() + 0.5,
            },
        ];

        if line_segments
            .iter()
            .map(|ls| {
                if self.next_idx > 1 {
                    ls.projected_point(pos).distance_squared(pos)
                } else {
                    LineSegment2 {
                        start: ls.start.xy(),
                        end: ls.end.xy(),
                    }
                    .projected_point(pos.xy())
                    .distance_squared(pos.xy())
                }
            })
            .reduce(|a, b| a.min(b))
            .is_some_and(|d| {
                d > if traversal_cfg.in_liquid {
                    traversal_cfg.node_tolerance * 5.0
                } else {
                    traversal_cfg.node_tolerance * 2.0
                }
                .powi(2)
            })
        {
            return Err(TraverseStop::InvalidPath);
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
            1.0 - (traversal_cfg.slow_factor * (1.0 - straight_factor)).min(0.9),
        ))
        .filter(|(bearing, _)| bearing.z < 2.1)
        .ok_or(TraverseStop::InvalidOutput)
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// How long the path we're trying to compute should be.
pub enum PathLength {
    #[default]
    Small,
    Medium,
    Long,
    Longest,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathState {
    /// There is no path.
    #[default]
    None,
    /// A non-complete path.
    Exhausted,
    /// In progress of computing a path.
    Pending,
    /// A complete path.
    Path,
}

/// A self-contained system that attempts to chase a moving target, only
/// performing pathfinding if necessary
#[derive(Default, Clone, Debug)]
pub struct Chaser {
    last_search_tgt: Option<Vec3<f32>>,
    /// `bool` indicates whether the Route is a complete route to the target
    ///
    /// `Vec3` is the target end pos
    route: Option<(Route, bool, Vec3<f32>)>,
    /// We use this hasher (FxHash) because:
    /// (1) we don't care about DDOS attacks (We can use FxHash);
    /// (2) we want this to be constant across compiles because of hot-reloading
    /// (Ruling out AAHash);
    ///
    /// The Vec3 is the astar's start position.
    astar: Option<(Astar<Node, FxBuildHasher>, Vec3<f32>)>,
    flee_from: Option<Vec3<f32>>,
    /// Whether to allow consideration of longer paths, npc will stand still
    /// while doing this.
    path_length: PathLength,

    /// The current state of the path.
    path_state: PathState,

    /// The last time the `chase` method was called.
    last_update_time: Option<Time>,

    /// (position, requested walk dir)
    recent_states: VecDeque<(Time, Vec3<f32>, Vec3<f32>)>,
}

impl Chaser {
    fn stuck_check(
        &mut self,
        pos: Vec3<f32>,
        bearing: Vec3<f32>,
        speed: f32,
        time: &Time,
    ) -> (Vec3<f32>, f32, bool) {
        /// The min amount of cached items.
        const MIN_CACHED_STATES: usize = 3;
        /// The max amount of cached items.
        const MAX_CACHED_STATES: usize = 10;
        /// Cache over 1 second.
        const CACHED_TIME_SPAN: f64 = 1.0;
        const TOLERANCE: f32 = 0.2;

        // We pop the first until there is only one element which was over
        // `CACHED_TIME_SPAN` seconds ago.
        while self.recent_states.len() > MIN_CACHED_STATES
            && self
                .recent_states
                .get(1)
                .is_some_and(|(t, ..)| time.0 - t.0 > CACHED_TIME_SPAN)
        {
            self.recent_states.pop_front();
        }

        if self.recent_states.len() < MAX_CACHED_STATES {
            self.recent_states.push_back((*time, pos, bearing * speed));

            if self.recent_states.len() >= MIN_CACHED_STATES
                && self
                    .recent_states
                    .front()
                    .is_some_and(|(t, ..)| time.0 - t.0 > CACHED_TIME_SPAN)
                && (bearing * speed).magnitude_squared() > 0.01
            {
                let average_pos = self
                    .recent_states
                    .iter()
                    .map(|(_, pos, _)| *pos)
                    .sum::<Vec3<f32>>()
                    * (1.0 / self.recent_states.len() as f32);
                let max_distance_sqr = self
                    .recent_states
                    .iter()
                    .map(|(_, pos, _)| pos.distance_squared(average_pos))
                    .reduce(|a, b| a.max(b));

                let average_speed = self
                    .recent_states
                    .iter()
                    .zip(self.recent_states.iter().skip(1).map(|(t, ..)| *t))
                    .map(|((t0, _, bearing), t1)| {
                        bearing.magnitude_squared() * (t1.0 - t0.0).powi(2) as f32
                    })
                    .sum::<f32>()
                    * (1.0 / self.recent_states.len() as f32);

                let is_stuck =
                    max_distance_sqr.is_some_and(|d| d < (average_speed * TOLERANCE).powi(2));

                let bearing = if is_stuck {
                    match rng().random_range(0..100u32) {
                        0..10 => -bearing,
                        10..20 => Vec3::new(bearing.y, bearing.x, bearing.z),
                        20..30 => Vec3::new(-bearing.y, bearing.x, bearing.z),
                        30..50 => {
                            if let Some((route, ..)) = &mut self.route {
                                route.next_idx = route.next_idx.saturating_sub(1);
                            }

                            bearing
                        },
                        50..60 => {
                            if let Some((route, ..)) = &mut self.route {
                                route.next_idx = route.next_idx.saturating_sub(2);
                            }

                            bearing
                        },
                        _ => bearing,
                    }
                } else {
                    bearing
                };

                return (bearing, speed, is_stuck);
            }
        }
        (bearing, speed, false)
    }

    fn reset(&mut self) {
        self.route = None;
        self.astar = None;
        self.last_search_tgt = None;
        self.path_length = Default::default();
        self.flee_from = None;
    }

    /// Returns bearing and speed
    /// Bearing is a `Vec3<f32>` dictating the direction of movement
    /// Speed is an f32 between 0.0 and 1.0
    pub fn chase<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        tgt: Vec3<f32>,
        traversal_cfg: TraversalConfig,
        time: &Time,
    ) -> Option<(Vec3<f32>, f32, bool)>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        span!(_guard, "chase", "Chaser::chase");
        self.last_update_time = Some(*time);
        // If we're already close to the target then there's nothing to do
        if ((pos - tgt) * Vec3::new(1.0, 1.0, 2.0)).magnitude_squared()
            < traversal_cfg.min_tgt_dist.powi(2)
        {
            self.reset();
            return None;
        }

        let d = tgt.distance_squared(pos);

        // Check if the current route is no longer valid.
        if let Some(end) = self.route.as_ref().map(|(_, _, end)| *end)
            && self.flee_from.is_none()
            && self.path_length < PathLength::Longest
            && d < tgt.distance_squared(end)
        {
            self.path_length = Default::default();
            self.route = None;
        }

        // If we're closer than the designated `flee_from` position, we ignore
        // that.
        if self.flee_from.is_some_and(|p| d < p.distance_squared(tgt)) {
            self.route = None;
            self.flee_from = None;
            self.astar = None;
            self.path_length = Default::default();
        }

        // Find a route if we don't have one.
        if self.route.is_none() {
            // Reset astar if last tgt is too far from tgt.
            if self
                .last_search_tgt
                .is_some_and(|last_tgt| tgt.distance_squared(last_tgt) > 2.0)
            {
                self.astar = None;
            }
            match find_path(
                &mut self.astar,
                vol,
                pos,
                tgt,
                &traversal_cfg,
                self.path_length,
                self.flee_from,
            ) {
                PathResult::Pending => {
                    self.path_state = PathState::Pending;
                },
                PathResult::None(path) => {
                    self.path_state = PathState::None;
                    self.route = Some((Route { path, next_idx: 0 }, false, tgt));
                },
                PathResult::Exhausted(path) => {
                    self.path_state = PathState::Exhausted;
                    self.route = Some((Route { path, next_idx: 0 }, false, tgt));
                },
                PathResult::Path(path, _) => {
                    self.flee_from = None;
                    self.path_state = PathState::Path;
                    self.path_length = Default::default();
                    self.route = Some((Route { path, next_idx: 0 }, true, tgt));
                },
            }

            self.last_search_tgt = Some(tgt);
        }

        if let Some((route, ..)) = &mut self.route {
            let res = route.traverse(vol, pos, vel, &traversal_cfg);

            // None either means we're done, or can't continue, either way we don't care
            // about that route anymore.
            if let Err(e) = &res {
                self.route = None;
                match e {
                    TraverseStop::InvalidOutput => {
                        return Some(self.stuck_check(
                            pos,
                            (tgt - pos).try_normalized().unwrap_or(Vec3::unit_x()),
                            1.0,
                            time,
                        ));
                    },
                    TraverseStop::InvalidPath => {
                        // If the path is invalid, blocks along the path have most likely changed,
                        // so reset the astar.
                        self.astar = None;
                    },
                    TraverseStop::Done => match self.path_state {
                        PathState::None => {
                            return Some(self.stuck_check(
                                pos,
                                (tgt - pos).try_normalized().unwrap_or_default(),
                                1.0,
                                time,
                            ));
                        },
                        PathState::Exhausted => {
                            // Upgrade path length if path is exhausted and we're at the same
                            // position.
                            if self.astar.as_ref().is_some_and(|(.., start)| {
                                start.distance_squared(pos) < traversal_cfg.node_tolerance.powi(2)
                            }) {
                                match self.path_length {
                                    PathLength::Small => {
                                        self.path_length = PathLength::Medium;
                                    },
                                    PathLength::Medium => {
                                        self.path_length = PathLength::Long;
                                    },
                                    PathLength::Long => {
                                        self.path_length = PathLength::Longest;
                                    },
                                    PathLength::Longest => {
                                        self.flee_from = Some(pos);
                                        self.astar = None;
                                    },
                                }
                            } else {
                                self.astar = None;
                            }
                        },
                        PathState::Pending | PathState::Path => {},
                    },
                }
            }

            let (bearing, speed) = res.ok()?;

            return Some(self.stuck_check(pos, bearing, speed, time));
        }

        None
    }

    pub fn get_route(&self) -> Option<&Route> { self.route.as_ref().map(|(r, ..)| r) }

    pub fn last_target(&self) -> Option<Vec3<f32>> { self.last_search_tgt }

    pub fn state(&self) -> (PathLength, PathState) { (self.path_length, self.path_state) }

    pub fn last_update_time(&self) -> Time {
        self.last_update_time.unwrap_or(Time(f64::NEG_INFINITY))
    }
}

fn walkable<V>(vol: &V, pos: Vec3<i32>, is_target_loaded: bool) -> bool
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let mut below_z = 1;
    // We loop downwards
    let below = loop {
        if let Some(block) = vol.get(pos - Vec3::unit_z() * below_z).ok().copied() {
            if block.is_solid() || block.is_liquid() {
                break block;
            }

            below_z += 1;

            if below_z > Block::MAX_HEIGHT.ceil() as i32 {
                break Block::empty();
            }
        } else if is_target_loaded {
            break Block::empty();
        } else {
            // If not loaded assume we can walk there.
            break Block::new(crate::terrain::BlockKind::Misc, Default::default());
        }
    };

    let a = vol.get(pos).ok().copied().unwrap_or_else(Block::empty);
    let b = vol
        .get(pos + Vec3::unit_z())
        .ok()
        .copied()
        .unwrap_or_else(Block::empty);

    let on_ground = (below_z == 1 && below.is_filled())
        || below.get_sprite().is_some_and(|sprite| {
            sprite
                .solid_height()
                .is_some_and(|h| ((below_z - 1) as f32) < h && h <= below_z as f32)
        });
    let in_liquid = a.is_liquid();
    (on_ground || in_liquid) && !a.is_solid() && !b.is_solid()
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Node {
    pos: Vec3<i32>,
    last_dir: Vec2<i32>,
    last_dir_count: u32,
}

/// Attempt to search for a path to a target, returning the path (if one was
/// found) and whether it is complete (reaches the target)
///
/// If `flee_from` is `Some` this will attempt to both walk away from that
/// position and towards the target.
fn find_path<V>(
    astar: &mut Option<(Astar<Node, FxBuildHasher>, Vec3<f32>)>,
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    traversal_cfg: &TraversalConfig,
    path_length: PathLength,
    flee_from: Option<Vec3<f32>>,
) -> PathResult<Vec3<i32>>
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let is_walkable = |pos: &Vec3<i32>| walkable(vol, *pos, traversal_cfg.is_target_loaded);
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

    // Find walkable ground for start and end.
    let (start, end) = match (
        get_walkable_z(startf.map(|e| e.floor() as i32)),
        get_walkable_z(endf.map(|e| e.floor() as i32)),
    ) {
        (Some(start), Some(end)) => (start, end),

        // Special case for partially loaded path finding
        (Some(start), None) if !traversal_cfg.is_target_loaded => {
            (start, endf.map(|e| e.floor() as i32))
        },

        _ => return PathResult::None(Path::default()),
    };

    let heuristic = |node: &Node| {
        let diff = end.as_::<f32>() - node.pos.as_::<f32>();
        let d = diff.magnitude();

        d - flee_from.map_or(0.0, |p| {
            let ndiff = p - node.pos.as_::<f32>() - 0.5;
            let nd = ndiff.magnitude();
            nd.sqrt() * ((diff / d).dot(ndiff / nd) + 0.1).max(0.0) * 10.0
        })
    };
    let transition = |a: Node, b: Node| {
        1.0
            // Discourage travelling in the same direction for too long: this encourages
            // turns to be spread out along a path, more closely approximating a straight
            // line toward the target.
            + b.last_dir_count as f32 * 0.01
            // Penalise jumping
            + (b.pos.z - a.pos.z + 1).max(0) as f32 * 2.0
    };
    let neighbors = |node: &Node| {
        let node = *node;
        let pos = node.pos;
        const DIRS: [Vec3<i32>; 9] = [
            Vec3::new(0, 1, 0), // Forward
            Vec3::new(0, 1, 1), // Forward upward
            // Vec3::new(0, 1, -1),  // Forward downward
            // Vec3::new(0, 1, -2),  // Forward downwardx2
            Vec3::new(1, 0, 0), // Right
            Vec3::new(1, 0, 1), // Right upward
            // Vec3::new(1, 0, -1),  // Right downward
            // Vec3::new(1, 0, -2),  // Right downwardx2
            Vec3::new(0, -1, 0), // Backwards
            Vec3::new(0, -1, 1), // Backward Upward
            // Vec3::new(0, -1, -1), // Backward downward
            // Vec3::new(0, -1, -2), // Backward downwardx2
            Vec3::new(-1, 0, 0), // Left
            Vec3::new(-1, 0, 1), // Left upward
            // Vec3::new(-1, 0, -1), // Left downward
            // Vec3::new(-1, 0, -2), // Left downwardx2
            Vec3::new(0, 0, -1), // Downwards
        ];

        const JUMPS: [Vec3<i32>; 4] = [
            Vec3::new(0, 1, 2),  // Forward Upwardx2
            Vec3::new(1, 0, 2),  // Right Upwardx2
            Vec3::new(0, -1, 2), // Backward Upwardx2
            Vec3::new(-1, 0, 2), // Left Upwardx2
        ];

        /// The cost of falling a block.
        const FALL_COST: f32 = 1.5;

        let walkable = [
            (is_walkable(&(pos + Vec3::new(1, 0, 0))), Vec3::new(1, 0, 0)),
            (
                is_walkable(&(pos + Vec3::new(-1, 0, 0))),
                Vec3::new(-1, 0, 0),
            ),
            (is_walkable(&(pos + Vec3::new(0, 1, 0))), Vec3::new(0, 1, 0)),
            (
                is_walkable(&(pos + Vec3::new(0, -1, 0))),
                Vec3::new(0, -1, 0),
            ),
        ];

        // Discourage walking alog walls/edges.
        let edge_cost = if path_length < PathLength::Medium {
            walkable.iter().any(|(w, _)| !*w) as i32 as f32
        } else {
            0.0
        };

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
                            .unwrap_or(traversal_cfg.is_target_loaded)
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
                            .unwrap_or(traversal_cfg.is_target_loaded))
                        && (dir.z < 2
                            || vol
                                .get(pos + Vec3::unit_z() * 3)
                                .map(|b| !b.is_solid())
                                .unwrap_or(traversal_cfg.is_target_loaded))
                        && (dir.z >= 0
                            || vol
                                .get(pos + *dir + Vec3::unit_z() * 2)
                                .map(|b| !b.is_solid())
                                .unwrap_or(traversal_cfg.is_target_loaded)))
            })
            .map(move |(pos, dir)| {
                let next_node = Node {
                    pos: pos + dir,
                    last_dir: dir.xy(),
                    last_dir_count: if node.last_dir == dir.xy() {
                        node.last_dir_count + 1
                    } else {
                        0
                    },
                };

                (
                    next_node,
                    transition(node, next_node) + if dir.z == 0 { edge_cost } else { 0.0 },
                )
            })
            // Falls
            .chain(walkable.into_iter().filter_map(move |(w, dir)| {
                let pos = pos + dir;
                if w ||
                    vol.get(pos).map(|b| b.is_solid()).unwrap_or(true) ||
                    vol.get(pos + Vec3::unit_z()).map(|b| b.is_solid()).unwrap_or(true) {
                    return None;
                }

                let down = (1..12).find(|i| is_walkable(&(pos - Vec3::unit_z() * *i)))?;

                let next_node = Node {
                    pos: pos - Vec3::unit_z() * down,
                    last_dir: dir.xy(),
                    last_dir_count: 0,
                };

                // Falling costs a lot.
                Some((next_node, match down {
                    1..=2 => {
                        transition(node, next_node)
                    }
                    _ => FALL_COST * (down - 2) as f32,
                }))
            }))
        // .chain(
        //     DIAGONALS
        //         .iter()
        //         .filter(move |(dir, [a, b])| {
        //             is_walkable(&(pos + *dir)) && walkable[*a] &&
        // walkable[*b]         })
        //         .map(move |(dir, _)| pos + *dir),
        // )
    };

    let satisfied = |node: &Node| node.pos == end;

    if astar
        .as_ref()
        .is_some_and(|(_, start)| start.distance_squared(startf) > 4.0)
    {
        *astar = None;
    }
    let max_iters = match path_length {
        PathLength::Small => 500,
        PathLength::Medium => 5000,
        PathLength::Long => 25_000,
        PathLength::Longest => 75_000,
    };

    let (astar, _) = astar.get_or_insert_with(|| {
        (
            Astar::new(
                max_iters,
                Node {
                    pos: start,
                    last_dir: Vec2::zero(),
                    last_dir_count: 0,
                },
                FxBuildHasher::default(),
            ),
            startf,
        )
    });

    astar.set_max_iters(max_iters);

    let path_result = astar.poll(
        match path_length {
            PathLength::Small => 250,
            PathLength::Medium => 400,
            PathLength::Long => 500,
            PathLength::Longest => 750,
        },
        heuristic,
        neighbors,
        satisfied,
    );

    path_result.map(|path| path.nodes.into_iter().map(|n| n.pos).collect())
}
// Enable when airbraking/sensible flight is a thing
#[cfg(feature = "rrt_pathfinding")]
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
        let path = vec![endf.map(|e| e.floor() as i32)];
        let connect = true;
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
        informed_rrt_connect(vol, startf, endf, is_traversable, radius)
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
#[cfg(feature = "rrt_pathfinding")]
fn informed_rrt_connect<V>(
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    is_valid_edge: impl Fn(&Vec3<f32>, &Vec3<f32>) -> bool,
    radius: f32,
) -> (Option<Path<Vec3<i32>>>, bool)
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    const MAX_POINTS: usize = 7000;
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
    let mut kdtree1: KdTree<f32, usize, 3, 32, u32> = KdTree::with_capacity(MAX_POINTS);
    let mut kdtree2: KdTree<f32, usize, 3, 32, u32> = KdTree::with_capacity(MAX_POINTS);

    // Add the start as the first node of the first k-d tree
    kdtree1.add(&[startf.x, startf.y, startf.z], node_index1);
    nodes1.push(startf);
    node_index1 += 1;

    // Add the end as the first node of the second k-d tree
    kdtree2.add(&[endf.x, endf.y, endf.z], node_index2);
    nodes2.push(endf);
    node_index2 += 1;

    let mut connection1_idx = 0;
    let mut connection2_idx = 0;

    let mut connect = false;

    // Scalar non-dimensional value that is proportional to the size of the
    // sample spheroid volume. This increases in value until a path is found.
    let mut search_parameter = 0.01;

    // Maximum of MAX_POINTS iterations
    for _i in 0..MAX_POINTS {
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
            .nearest_one::<SquaredEuclidean>(&[
                sampled_point1.x,
                sampled_point1.y,
                sampled_point1.z,
            ])
            .item;
        let nearest_index2 = kdtree2
            .nearest_one::<SquaredEuclidean>(&[
                sampled_point2.x,
                sampled_point2.y,
                sampled_point2.z,
            ])
            .item;
        let nearest1 = nodes1[nearest_index1];
        let nearest2 = nodes2[nearest_index2];

        // Extend toward the sampled point from the nearest node of each tree
        let new_point1 = nearest1 + (sampled_point1 - nearest1).normalized().map(|a| a * radius);
        let new_point2 = nearest2 + (sampled_point2 - nearest2).normalized().map(|a| a * radius);

        // Ensure the new nodes are valid/traversable
        if is_valid_edge(&nearest1, &new_point1) {
            kdtree1.add(&[new_point1.x, new_point1.y, new_point1.z], node_index1);
            nodes1.push(new_point1);
            parents1.insert(node_index1, nearest_index1);
            node_index1 += 1;
            // Check if the trees connect
            let NearestNeighbour {
                distance: check,
                item: index,
            } = kdtree2.nearest_one::<SquaredEuclidean>(&[
                new_point1.x,
                new_point1.y,
                new_point1.z,
            ]);
            if check < radius {
                let connection = nodes2[index];
                connection2_idx = index;
                nodes1.push(connection);
                connection1_idx = nodes1.len() - 1;
                parents1.insert(node_index1, node_index1 - 1);
                connect = true;
            }
        }

        // Repeat the validity check for the second tree
        if is_valid_edge(&nearest2, &new_point2) {
            kdtree2.add(&[new_point2.x, new_point2.y, new_point1.z], node_index2);
            nodes2.push(new_point2);
            parents2.insert(node_index2, nearest_index2);
            node_index2 += 1;
            // Again check for a connection
            let NearestNeighbour {
                distance: check,
                item: index,
            } = kdtree1.nearest_one::<SquaredEuclidean>(&[
                new_point2.x,
                new_point2.y,
                new_point1.z,
            ]);
            if check < radius {
                let connection = nodes1[index];
                connection1_idx = index;
                nodes2.push(connection);
                connection2_idx = nodes2.len() - 1;
                parents2.insert(node_index2, node_index2 - 1);
                connect = true;
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
            .nearest_one::<SquaredEuclidean>(&[endf.x, endf.y, endf.z])
            .item;
        // Attempt to pick a node other than the start node
        for _i in 0..3 {
            if current_node_index1 == 0
                || nodes1[current_node_index1].distance_squared(startf) < 4.0
            {
                if let Some(index) = parents1.values().choose(&mut rng()) {
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
    (Some(path.into_iter().collect()), connect)
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
#[cfg(feature = "rrt_pathfinding")]
pub fn point_on_prolate_spheroid(
    focus1: Vec3<f32>,
    focus2: Vec3<f32>,
    search_parameter: f32,
) -> Vec3<f32> {
    let mut rng = rng();
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
