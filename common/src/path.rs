use crate::{
    astar::{Astar, PathResult},
    terrain::Block,
    vol::{BaseVol, ReadVol},
};
use std::iter::FromIterator;
use vek::*;

// Path

#[derive(Default, Clone, Debug)]
pub struct Path<T> {
    nodes: Vec<T>,
}

impl<T> FromIterator<T> for Path<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            nodes: iter.into_iter().collect(),
        }
    }
}

impl<T> Path<T> {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn start(&self) -> Option<&T> {
        self.nodes.first()
    }

    pub fn end(&self) -> Option<&T> {
        self.nodes.last()
    }
}

// Route: A path that can be progressed along

#[derive(Default, Clone, Debug)]
pub struct Route {
    path: Path<Vec3<i32>>,
    next_idx: usize,
}

impl From<Path<Vec3<i32>>> for Route {
    fn from(path: Path<Vec3<i32>>) -> Self {
        Self { path, next_idx: 0 }
    }
}

impl Route {
    pub fn path(&self) -> &Path<Vec3<i32>> {
        &self.path
    }

    pub fn next(&self) -> Option<Vec3<i32>> {
        self.path.nodes.get(self.next_idx).copied()
    }

    pub fn is_finished(&self) -> bool {
        self.next().is_none()
    }

    pub fn traverse<V>(&mut self, vol: &V, pos: Vec3<f32>) -> Option<Vec3<f32>>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let next = self.next()?;
        if vol.get(next).map(|b| b.is_solid()).unwrap_or(false) {
            None
        } else {
            let next_tgt = next.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0);
            if next_tgt.distance_squared(pos) < 1.0f32.powf(2.0) {
                self.next_idx += 1;
            }
            Some(next_tgt - pos)
        }
    }
}

// Chaser: A self-contained system that attempts to chase a moving target

#[derive(Default, Clone, Debug)]
pub struct Chaser {
    last_search_tgt: Option<Vec3<f32>>,
    route: Route,
    astar: Option<Astar<Vec3<i32>>>,
}

impl Chaser {
    pub fn chase<V>(&mut self, vol: &V, pos: Vec3<f32>, tgt: Vec3<f32>) -> Option<Vec3<f32>>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let pos_to_tgt = pos.distance(tgt);

        if pos_to_tgt < 4.0 {
            return None;
        }

        let bearing = if let Some(end) = self.route.path().end().copied() {
            let end_to_tgt = end.map(|e| e as f32).distance(tgt);
            if end_to_tgt > pos_to_tgt * 0.3 + 5.0 {
                None
            } else {
                if rand::random::<f32>() < 0.005 {
                    // TODO: Only re-calculate route when we're stuck
                    self.route = Route::default();
                }

                self.route.traverse(vol, pos)
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

            Some(tgt - pos)
        }
    }
}

fn find_path<V>(
    astar: &mut Option<Astar<Vec3<i32>>>,
    vol: &V,
    start: Vec3<f32>,
    end: Vec3<f32>,
) -> Path<Vec3<i32>>
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let is_walkable = |pos: &Vec3<i32>| {
        vol.get(*pos - Vec3::new(0, 0, 1))
            .map(|b| b.is_solid())
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
        for i in 0..32 {
            let test_pos = pos + Vec3::unit_z() * z_incr;
            if is_walkable(&test_pos) {
                return Some(test_pos);
            }
            z_incr = -z_incr + if z_incr <= 0 { 1 } else { 0 };
        }
        None
    };

    let (start, end) = match (
        get_walkable_z(start.map(|e| e.floor() as i32)),
        get_walkable_z(end.map(|e| e.floor() as i32)),
    ) {
        (Some(start), Some(end)) => (start, end),
        _ => return Path::default(),
    };

    let heuristic = |pos: &Vec3<i32>| (pos.distance_squared(end) as f32).sqrt();
    let neighbors = |pos: &Vec3<i32>| {
        let pos = *pos;
        const dirs: [Vec3<i32>; 17] = [
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

        dirs.iter()
            .map(move |dir| pos + dir)
            .filter(move |pos| is_walkable(pos))
    };
    let transition = |_: &Vec3<i32>, _: &Vec3<i32>| 1.0;
    let satisfied = |pos: &Vec3<i32>| pos == &end;

    let mut new_astar = match astar.take() {
        None => {
            let max_iters = ((Vec2::<f32>::from(start).distance(Vec2::from(end)) + 10.0).powf(2.0)
                as usize)
                .min(25_000);
            Astar::new(max_iters, start, heuristic.clone())
        }
        Some(astar) => astar,
    };

    let path_result = new_astar.poll(30, heuristic, neighbors, transition, satisfied);

    *astar = Some(new_astar);

    match path_result {
        PathResult::Path(path) => {
            *astar = None;
            path
        }
        PathResult::Pending => Path::default(),
        _ => {
            *astar = None;
            Path::default()
        }
    }
}
