use crate::{
    astar::{Astar, PathResult},
    terrain::Block,
    vol::{BaseVol, ReadVol},
};
use hashbrown::hash_map::DefaultHashBuilder;
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

#[allow(clippy::len_without_is_empty)] // TODO: Pending review in #587
impl<T> Path<T> {
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
        on_ground: bool,
        traversal_tolerance: f32,
    ) -> Option<(Vec3<f32>, f32)>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let (next0, next1, next_tgt) = loop {
            let next0 = self
                .next(0)
                .unwrap_or_else(|| pos.map(|e| e.floor() as i32));

            // Stop using obstructed paths
            if vol.get(next0).map(|b| b.is_solid()).unwrap_or(false) {
                return None;
            }

            let next1 = self.next(1).unwrap_or(next0);
            let next_tgt = next0.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0);

            // Determine whether we're close enough to the next to to consider it completed
            if pos.xy().distance_squared(next_tgt.xy()) < traversal_tolerance.powf(2.0)
                && (pos.z - next_tgt.z > 1.2 || (pos.z - next_tgt.z > -0.2 && on_ground))
                && pos.z - next_tgt.z < 2.2
                && vel.z <= 0.0
                // Only consider the node reached if there's nothing solid between us and it
                && vol
                    .ray(pos + Vec3::unit_z() * 1.5, next_tgt + Vec3::unit_z() * 1.5)
                    .until(|block| block.is_solid())
                    .cast()
                    .0
                    > pos.distance(next_tgt) * 0.9
                && self.next_idx < self.path.len()
            {
                // Node completed, move on to the next one
                self.next_idx += 1;
            } else {
                // The next node hasn't been reached yet, use it as a target
                break (next0, next1, next_tgt);
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
            let lerp_block = |x, precision| Lerp::lerp(x, block_pos.xy().map(|e| e as f32), precision);

            (0..4)
                .filter_map(|i| {
                    let edge_line = LineSegment2 {
                        start: lerp_block((block_pos.xy() + corners[i]).map(|e| e as f32), precision),
                        end: lerp_block((block_pos.xy() + corners[i + 1]).map(|e| e as f32), precision),
                    };
                    intersect(vel_line, edge_line)
                        .filter(|intersect| intersect.clamped(
                            block_pos.xy().map(|e| e as f32),
                            block_pos.xy().map(|e| e as f32 + 1.0),
                        ).distance_squared(*intersect) < 0.001)
                })
                .min_by_key(|intersect: &Vec2<f32>| (intersect.distance_squared(vel_line.end) * 1000.0) as i32)
                .unwrap_or_else(|| (0..2)
                    .map(|i| (0..2).map(move |j| Vec2::new(i, j)))
                        .flatten()
                        .map(|rpos| block_pos + rpos)
                        .map(|block_pos| {
                            let block_posf = block_pos.xy().map(|e| e as f32);
                            let proj = vel_line.projected_point(block_posf);
                            let clamped = lerp_block(proj.clamped(
                                block_pos.xy().map(|e| e as f32),
                                block_pos.xy().map(|e| e as f32),
                            ), precision);

                            (proj.distance_squared(clamped), clamped)
                        })
                        .min_by_key(|(d2, _)| (d2 * 1000.0) as i32)
                        .unwrap()
                        .1)
        };

        let bez = CubicBezier2 {
            start: pos.xy(),
            ctrl0: pos.xy() + vel.xy().try_normalized().unwrap_or(Vec2::zero()) * 1.0,
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
            .unwrap_or(Vec2::zero());
        let straight_factor = next_dir
            .dot(vel.xy().try_normalized().unwrap_or(next_dir))
            .max(0.0)
            .powf(2.0);

        let bez = CubicBezier2 {
            start: pos.xy(),
            ctrl0: pos.xy() + vel.xy().try_normalized().unwrap_or(Vec2::zero()) * 1.0,
            ctrl1: align(next0, (1.0 - straight_factor * if (next0.z as f32 - pos.z).abs() < 0.25 { 1.0 } else { 0.0 }).max(0.1)),
            end: align(next1, 1.0),
        };

        let tgt2d = bez.evaluate(if (next0.z as f32 - pos.z).abs() < 0.25 { 0.25 } else { 0.5 });
        let tgt = Vec3::from(tgt2d) + Vec3::unit_z() * next_tgt.z;

        Some((
            tgt - pos,
            // Control the entity's speed to hopefully stop us falling off walls on sharp corners.
            // This code is very imperfect: it does its best but it can still fail for particularly
            // fast entities.
            straight_factor * 0.75 + 0.25,
        ))
            .filter(|(bearing, _)| bearing.z < 2.1)
    }
}

/// A self-contained system that attempts to chase a moving target, only
/// performing pathfinding if necessary
#[derive(Default, Clone, Debug)]
pub struct Chaser {
    last_search_tgt: Option<Vec3<f32>>,
    route: Option<Route>,
    /// We use this hasher (AAHasher) because:
    /// (1) we care about DDOS attacks (ruling out FxHash);
    /// (2) we don't care about determinism across computers (we can use
    /// AAHash).
    astar: Option<Astar<Vec3<i32>, DefaultHashBuilder>>,
}

impl Chaser {
    pub fn chase<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        on_ground: bool,
        tgt: Vec3<f32>,
        min_dist: f32,
        traversal_tolerance: f32,
    ) -> Option<(Vec3<f32>, f32)>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let pos_to_tgt = pos.distance(tgt);

        // If we're already close to the target then there's nothing to do
        if ((pos - tgt) * Vec3::new(1.0, 1.0, 2.0)).magnitude_squared() < min_dist.powf(2.0) {
            self.route = None;
            return None;
        }

        let bearing = if let Some(end) = self.route.as_ref().and_then(|r| r.path().end().copied()) {
            let end_to_tgt = end.map(|e| e as f32).distance(tgt);
            // If the target has moved significantly since the path was generated then it's
            // time to search for a new path. Also, do this randomly from time
            // to time to avoid any edge cases that cause us to get stuck. In
            // theory this shouldn't happen, but in practice the world is full
            // of unpredictable obstacles that are more than willing to mess up
            // our day. TODO: Come up with a better heuristic for this
            if end_to_tgt > pos_to_tgt * 0.3 + 5.0
            /* || thread_rng().gen::<f32>() < 0.005 */
            {
                None
            } else {
                self.route
                    .as_mut()
                    .and_then(|r| r.traverse(vol, pos, vel, on_ground, traversal_tolerance))
                    // In theory this filter isn't needed, but in practice agents often try to take
                    // stale paths that start elsewhere. This code makes sure that we're only using
                    // paths that start near us, avoiding the agent doubling back to chase a stale
                    // path.
                    .filter(|(bearing, _)| bearing.xy()
                        .magnitude_squared() < (traversal_tolerance * 3.0).powf(2.0))
            }
        } else {
            None
        };

        if let Some((bearing, speed)) = bearing {
            Some((bearing, speed))
        } else {
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
                let (start_pos, path) = find_path(&mut self.astar, vol, pos, tgt);
                // Don't use a stale path
                if start_pos.distance_squared(pos) < 4.0f32.powf(2.0) {
                    self.route = path.map(Route::from);
                } else {
                    self.route = None;
                }
            }

            Some(((tgt - pos) * Vec3::new(1.0, 1.0, 0.0), 0.75))
        }
    }
}

#[allow(clippy::float_cmp)] // TODO: Pending review in #587
fn find_path<V>(
    astar: &mut Option<Astar<Vec3<i32>, DefaultHashBuilder>>,
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
) -> (Vec3<f32>, Option<Path<Vec3<i32>>>)
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let is_walkable = |pos: &Vec3<i32>| {
        vol.get(*pos - Vec3::new(0, 0, 1))
            .map(|b| b.is_solid() && b.get_height() == 1.0)
            .unwrap_or(false)
            && vol
                .get(*pos + Vec3::new(0, 0, 0))
                .map(|b| !b.is_solid())
                .unwrap_or(true)
            && vol
                .get(*pos + Vec3::new(0, 0, 1))
                .map(|b| !b.is_solid())
                .unwrap_or(true)
    };
    let get_walkable_z = |pos| {
        let mut z_incr = 0;
        for _ in 0..32 {
            let test_pos = pos + Vec3::unit_z() * z_incr;
            if is_walkable(&test_pos) {
                return Some(test_pos);
            }
            z_incr = -z_incr + if z_incr <= 0 { 1 } else { 0 };
        }
        None
    };

    let (start, end) = match (
        get_walkable_z(startf.map(|e| e.floor() as i32)),
        get_walkable_z(endf.map(|e| e.floor() as i32)),
    ) {
        (Some(start), Some(end)) => (start, end),
        _ => return (startf, None),
    };

    let heuristic = |pos: &Vec3<i32>| (pos.distance_squared(end) as f32).sqrt();
    let neighbors = |pos: &Vec3<i32>| {
        let pos = *pos;
        const DIRS: [Vec3<i32>; 17] = [
            Vec3::new(0, 1, 0),   // Forward
            Vec3::new(0, 1, 1),   // Forward upward
            Vec3::new(0, 1, 2),   // Forward Upwardx2
            Vec3::new(0, 1, -1),  // Forward downward
            Vec3::new(1, 0, 0),   // Right
            Vec3::new(1, 0, 1),   // Right upward
            Vec3::new(1, 0, 2),   // Right Upwardx2
            Vec3::new(1, 0, -1),  // Right downward
            Vec3::new(0, -1, 0),  // Backwards
            Vec3::new(0, -1, 1),  // Backward Upward
            Vec3::new(0, -1, 2),  // Backward Upwardx2
            Vec3::new(0, -1, -1), // Backward downward
            Vec3::new(-1, 0, 0),  // Left
            Vec3::new(-1, 0, 1),  // Left upward
            Vec3::new(-1, 0, 2),  // Left Upwardx2
            Vec3::new(-1, 0, -1), // Left downward
            Vec3::new(0, 0, -1),  // Downwards
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
            .map(move |dir| (pos, dir))
            .filter(move |(pos, dir)| {
                is_walkable(pos)
                    && is_walkable(&(*pos + **dir))
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

    let crow_line = LineSegment2 {
        start: startf.xy(),
        end: endf.xy(),
    };

    let transition = |a: &Vec3<i32>, b: &Vec3<i32>| {
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

    (startf, match path_result {
        PathResult::Path(path) => {
            *astar = None;
            Some(path)
        },
        PathResult::None(path) => {
            *astar = None;
            Some(path)
        },
        PathResult::Exhausted(path) => {
            *astar = None;
            Some(path)
        },
        PathResult::Pending => None,
    })
}
