use crate::{
    pathfinding::WorldPath,
    terrain::Block,
    vol::{BaseVol, ReadVol},
};
use std::iter::FromIterator;
use vek::*;

// Path

#[derive(Default, Clone, Debug)]
pub struct Path {
    nodes: Vec<Vec3<i32>>,
}

impl FromIterator<Vec3<i32>> for Path {
    fn from_iter<I: IntoIterator<Item = Vec3<i32>>>(iter: I) -> Self {
        Self {
            nodes: iter.into_iter().collect(),
        }
    }
}

impl Path {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn start(&self) -> Option<Vec3<i32>> {
        self.nodes.first().copied()
    }

    pub fn end(&self) -> Option<Vec3<i32>> {
        self.nodes.last().copied()
    }
}

// Route: A path that can be progressed along

#[derive(Default, Clone, Debug)]
pub struct Route {
    path: Path,
    next_idx: usize,
}

impl From<Path> for Route {
    fn from(path: Path) -> Self {
        Self { path, next_idx: 0 }
    }
}

impl Route {
    pub fn path(&self) -> &Path {
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
    route: Route,
}

impl Chaser {
    pub fn chase<V>(&mut self, vol: &V, tgt: Vec3<f32>, pos: Vec3<f32>) -> Option<Vec3<f32>>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let pos_to_tgt = pos.distance(tgt);

        if pos_to_tgt < 4.0 {
            return None;
        }

        let bearing = if let Some(end) = self.route.path().end() {
            let end_to_tgt = end.map(|e| e as f32).distance(tgt);
            if end_to_tgt > pos_to_tgt * 0.3 + 5.0 {
                None
            } else {
                self.route.traverse(vol, pos)
            }
        } else {
            None
        };

        // TODO: What happens when we get stuck?
        if let Some(bearing) = bearing {
            Some(bearing)
        } else {
            let path: Path = WorldPath::find(vol, pos, tgt)
                .ok()
                .and_then(|wp| wp.path.map(|nodes| nodes.into_iter().rev()))
                .into_iter()
                .flatten()
                .collect();

            self.route = path.into();

            Some(tgt - pos)
        }
    }
}
