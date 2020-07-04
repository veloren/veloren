use crate::{
    astar::{Astar, PathResult},
    terrain::Block,
    vol::{BaseVol, ReadVol},
};
use hashbrown::hash_map::DefaultHashBuilder;
use rand::{thread_rng, Rng};
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

    pub fn next(&self) -> Option<Vec3<i32>> { self.path.nodes.get(self.next_idx).copied() }

    pub fn is_finished(&self) -> bool { self.next().is_none() }

    pub fn traverse<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        traversal_tolerance: f32,
    ) -> Option<Vec3<f32>>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let next = self.next()?;
        if vol.get(next).map(|b| b.is_solid()).unwrap_or(false) {
            None
        } else {
            let next_tgt = next.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0);
            if ((pos - (next_tgt + Vec3::unit_z() * 0.5)) * Vec3::new(1.0, 1.0, 0.3))
                .magnitude_squared()
                < (traversal_tolerance * 2.0).powf(2.0)
            {
                self.next_idx += 1;
            }
            Some(next_tgt - pos)
        }
    }
}

/// A self-contained system that attempts to chase a moving target, only
/// performing pathfinding if necessary
#[derive(Default, Clone, Debug)]
pub struct Chaser {
    last_search_tgt: Option<Vec3<f32>>,
    route: Route,
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
        tgt: Vec3<f32>,
        min_dist: f32,
        traversal_tolerance: f32,
    ) -> Option<Vec3<f32>>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let pos_to_tgt = pos.distance(tgt);

        if ((pos - tgt) * Vec3::new(1.0, 1.0, 0.15)).magnitude_squared() < min_dist.powf(2.0) {
            return None;
        }

        let bearing = if let Some(end) = self.route.path().end().copied() {
            let end_to_tgt = end.map(|e| e as f32).distance(tgt);
            if end_to_tgt > pos_to_tgt * 0.3 + 5.0 {
                None
            } else {
                if thread_rng().gen::<f32>() < 0.005 {
                    // TODO: Only re-calculate route when we're stuck
                    self.route = Route::default();
                }

                self.route.traverse(vol, pos, traversal_tolerance)
            }
        } else {
            None
        };

        // TODO: What happens when we get stuck?
        if let Some(bearing) = bearing {
            Some(bearing)
        } else {
            if self
                .last_search_tgt
                .map(|last_tgt| last_tgt.distance(tgt) > pos_to_tgt * 0.15 + 5.0)
                .unwrap_or(true)
            {
                self.route = find_path(&mut self.astar, vol, pos, tgt).into();
            }

            Some((tgt - pos) * Vec3::new(1.0, 1.0, 0.0))
        }
    }
}

#[allow(clippy::float_cmp)] // TODO: Pending review in #587
fn find_path<V>(
    astar: &mut Option<Astar<Vec3<i32>, DefaultHashBuilder>>,
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
) -> Path<Vec3<i32>>
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
        _ => return Path::default(),
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

        let walkable = [
            is_walkable(&(pos + Vec3::new(1, 0, 0))),
            is_walkable(&(pos + Vec3::new(-1, 0, 0))),
            is_walkable(&(pos + Vec3::new(0, 1, 0))),
            is_walkable(&(pos + Vec3::new(0, -1, 0))),
        ];

        const DIAGONALS: [(Vec3<i32>, [usize; 2]); 8] = [
            (Vec3::new(1, 1, 0), [0, 2]),
            (Vec3::new(-1, 1, 0), [1, 2]),
            (Vec3::new(1, -1, 0), [0, 3]),
            (Vec3::new(-1, -1, 0), [1, 3]),
            (Vec3::new(1, 1, 1), [0, 2]),
            (Vec3::new(-1, 1, 1), [1, 2]),
            (Vec3::new(1, -1, 1), [0, 3]),
            (Vec3::new(-1, -1, 1), [1, 3]),
        ];

        DIRS.iter()
            .map(move |dir| (pos, dir))
            .filter(move |(pos, dir)| {
                is_walkable(pos)
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
            .chain(
                DIAGONALS
                    .iter()
                    .filter(move |(dir, [a, b])| {
                        is_walkable(&(pos + *dir)) && walkable[*a] && walkable[*b]
                    })
                    .map(move |(dir, _)| pos + *dir),
            )
    };
    let transition = |_: &Vec3<i32>, _: &Vec3<i32>| 1.0;
    let satisfied = |pos: &Vec3<i32>| pos == &end;

    let mut new_astar = match astar.take() {
        None => Astar::new(20_000, start, heuristic, DefaultHashBuilder::default()),
        Some(astar) => astar,
    };

    let path_result = new_astar.poll(60, heuristic, neighbors, transition, satisfied);

    *astar = Some(new_astar);

    match path_result {
        PathResult::Path(path) => {
            *astar = None;
            path
        },
        PathResult::None(path) => {
            *astar = None;
            path
        },
        PathResult::Exhausted(path) => {
            *astar = None;
            path
        },
        PathResult::Pending => Path::default(),
    }
}
