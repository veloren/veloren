use crate::{
    astar::astar,
    pathfinding::WorldPath,
    vol::{ReadVol, RectRasterableVol},
    volumes::vol_grid_2d::VolGrid2d,
};

use std::fmt::Debug;
use vek::*;

#[derive(Clone, Debug, Default)]
pub struct ChunkPath {
    pub from: Vec3<f32>,
    pub dest: Vec3<f32>,
    pub chunk_path: Option<Vec<Vec2<i32>>>,
}

impl ChunkPath {
    pub fn new<V: RectRasterableVol + ReadVol + Debug>(
        vol: &VolGrid2d<V>,
        from: Vec3<f32>,
        dest: Vec3<f32>,
    ) -> Self {
        let ifrom: Vec3<i32> = Vec3::from(from.map(|e| e.floor() as i32));
        let idest: Vec3<i32> = Vec3::from(dest.map(|e| e.floor() as i32));

        let start_chunk = vol.pos_key(ifrom);
        let end_chunk = vol.pos_key(idest);

        let chunk_path = astar(
            start_chunk,
            end_chunk,
            chunk_euclidean_distance,
            |pos| ChunkPath::chunk_get_neighbors(vol, pos),
            chunk_transition_cost,
        );

        Self {
            from,
            dest,
            chunk_path,
        }
    }

    pub fn chunk_get_neighbors<V: RectRasterableVol + ReadVol + Debug>(
        _vol: &VolGrid2d<V>,
        pos: &Vec2<i32>,
    ) -> impl Iterator<Item = Vec2<i32>> {
        let directions = vec![
            Vec2::new(1, 0),  // Right chunk
            Vec2::new(-1, 0), // Left chunk
            Vec2::new(0, 1),  // Top chunk
            Vec2::new(0, -1), // Bottom chunk
        ];

        let mut neighbors = Vec::new();
        for x in -2..3 {
            for y in -2..3 {
                neighbors.push(pos + Vec2::new(x, y));
            }
        }

        //let neighbors: Vec<Vec2<i32>> = directions.into_iter().map(|dir| dir +
        // pos).collect();

        neighbors.into_iter()
    }

    pub fn worldpath_get_neighbors<V: RectRasterableVol + ReadVol + Debug>(
        &mut self,
        vol: &VolGrid2d<V>,
        pos: Vec3<i32>,
    ) -> impl Iterator<Item = Vec3<i32>> {
        let directions = vec![
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
        ];

        let neighbors: Vec<Vec3<i32>> = directions
            .into_iter()
            .map(|dir| dir + pos)
            .filter(|new_pos| self.is_valid_space(vol, *new_pos))
            .collect();
        neighbors.into_iter()
    }

    pub fn is_valid_space<V: RectRasterableVol + ReadVol + Debug>(
        &mut self,
        vol: &VolGrid2d<V>,
        pos: Vec3<i32>,
    ) -> bool {
        let is_walkable_position = WorldPath::is_walkable_space(vol, pos);
        let mut is_within_chunk = false;
        match self.chunk_path.clone() {
            Some(chunk_path) => {
                is_within_chunk = chunk_path
                    .iter()
                    .any(|new_pos| new_pos.cmpeq(&vol.pos_key(pos)).iter().all(|e| *e));
            },
            _ => {
                //println!("No chunk path");
            },
        }
        return is_walkable_position && is_within_chunk;
    }

    pub fn get_worldpath<V: RectRasterableVol + ReadVol + Debug>(
        &mut self,
        vol: &VolGrid2d<V>,
    ) -> Result<WorldPath, ()> {
        let wp = WorldPath::new(vol, self.from, self.dest, |vol, pos| {
            self.worldpath_get_neighbors(vol, pos)
        });
        //println!("Fetching world path from hierarchical path: {:?}", wp);
        wp
    }
}

pub fn chunk_euclidean_distance(start: &Vec2<i32>, end: &Vec2<i32>) -> f32 {
    let istart = start.map(|e| e as f32);
    let iend = end.map(|e| e as f32);
    istart.distance(iend)
}

pub fn chunk_transition_cost(_start: &Vec2<i32>, _end: &Vec2<i32>) -> f32 { 1.0f32 }
