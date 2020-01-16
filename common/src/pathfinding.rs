use crate::comp::{ControllerInputs, Pos};
use crate::{
    astar::astar,
    vol::{ReadVol, Vox},
};
use vek::*;

#[derive(Clone, Debug, Default)]
pub struct WorldPath {
    pub from: Vec3<f32>,
    pub dest: Vec3<f32>,
    pub path: Option<Vec<Vec3<i32>>>,
}

impl WorldPath {
    pub fn new<V: ReadVol, I>(
        vol: &V,
        from: Vec3<f32>,
        dest: Vec3<f32>,
        get_neighbors: impl FnMut(&V, &Vec3<i32>) -> I,
    ) -> Self
    where
        I: IntoIterator<Item = Vec3<i32>>,
    {
        let ifrom: Vec3<i32> = Vec3::from(from.map(|e| e.floor() as i32));
        let idest: Vec3<i32> = Vec3::from(dest.map(|e| e.floor() as i32));
        let path = WorldPath::get_path(vol, ifrom, idest, get_neighbors);

        Self { from, dest, path }
    }

    pub fn get_path<V: ReadVol, I>(
        vol: &V,
        from: Vec3<i32>,
        dest: Vec3<i32>,
        mut get_neighbors: impl FnMut(&V, &Vec3<i32>) -> I,
    ) -> Option<Vec<Vec3<i32>>>
    where
        I: IntoIterator<Item = Vec3<i32>>,
    {
        let new_start = WorldPath::get_z_walkable_space(vol, from);
        let new_dest = WorldPath::get_z_walkable_space(vol, dest);

        if let (Some(new_start), Some(new_dest)) = (new_start, new_dest) {
            astar(
                new_start,
                new_dest,
                euclidean_distance,
                |pos| get_neighbors(vol, pos),
                transition_cost,
            )
        } else {
            None
        }
    }

    fn get_z_walkable_space<V: ReadVol>(vol: &V, pos: Vec3<i32>) -> Option<Vec3<i32>> {
        if WorldPath::is_walkable_space(vol, pos) {
            return Some(pos);
        }

        let mut cur_pos_below = pos.clone();
        while !WorldPath::is_walkable_space(vol, cur_pos_below) && cur_pos_below.z > 0 {
            cur_pos_below.z -= 1;
        }

        let max_z = 1000;
        let mut cur_pos_above = pos.clone();
        while !WorldPath::is_walkable_space(vol, cur_pos_above) && cur_pos_above.z <= max_z {
            cur_pos_above.z += 1;
        }

        if cur_pos_below.z > 0 {
            Some(cur_pos_below)
        } else if cur_pos_above.z < max_z {
            Some(cur_pos_above)
        } else {
            None
        }
    }

    pub fn is_walkable_space<V: ReadVol>(vol: &V, pos: Vec3<i32>) -> bool {
        if let (Ok(voxel), Ok(upper_neighbor), Ok(upper_neighbor2)) = (
            vol.get(pos),
            vol.get(pos + Vec3::new(0, 0, 1)),
            vol.get(pos + Vec3::new(0, 0, 2)),
        ) {
            !voxel.is_empty() && upper_neighbor.is_empty() && upper_neighbor2.is_empty()
        } else {
            false
        }
    }

    pub fn get_neighbors<V: ReadVol>(
        vol: &V,
        pos: &Vec3<i32>,
    ) -> impl IntoIterator<Item = Vec3<i32>> {
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
            Vec3::new(0, 0, -1),  // Downwards
        ];

        let neighbors: Vec<Vec3<i32>> = directions
            .into_iter()
            .map(|dir| dir + pos)
            .filter(|new_pos| Self::is_walkable_space(vol, *new_pos))
            .collect();

        neighbors.into_iter()
    }

    pub fn move_along_path<V: ReadVol>(
        &mut self,
        vol: &V,
        pos: &Pos,
        inputs: &mut ControllerInputs,
        is_destination: impl Fn(Vec3<i32>, Vec3<i32>) -> bool,
        found_destination: impl FnOnce(),
    ) {
        // No path available
        if self.path == None {
            return;
        }

        let ipos = pos.0.map(|e| e.floor() as i32);
        let idest = self.dest.map(|e| e.floor() as i32);

        // We have reached the end of the path
        if is_destination(ipos, idest) {
            found_destination();
        }

        if let Some(mut block_path) = self.path.clone() {
            if let Some(next_pos) = block_path.clone().last() {
                if self.path_is_blocked(vol) {
                    self.path = WorldPath::get_path(vol, ipos, idest, WorldPath::get_neighbors);
                }

                if Vec2::<i32>::from(ipos) == Vec2::<i32>::from(*next_pos) {
                    block_path.pop();
                    self.path = Some(block_path);
                }

                let move_dir = Vec2::<i32>::from(next_pos - ipos);

                // Move the input towards the next area on the path
                inputs.move_dir = Vec2::<f32>::new(move_dir.x as f32, move_dir.y as f32);

                // Need to jump to continue
                if next_pos.z >= ipos.z + 1 {
                    inputs.jump.set_state(true);
                }

                // Need to glide
                let min_z_glide_height = 3;
                if next_pos.z - min_z_glide_height < ipos.z {
                    inputs.glide.set_state(true);
                }
            }
        }
    }

    pub fn path_is_blocked<V: ReadVol>(&self, vol: &V) -> bool {
        match self.path.clone() {
            Some(path) => path
                .iter()
                .any(|pos| !WorldPath::is_walkable_space(vol, *pos)),
            _ => false,
        }
    }
}

pub fn euclidean_distance(start: &Vec3<i32>, end: &Vec3<i32>) -> f32 {
    start.map(|e| e as f32).distance(end.map(|e| e as f32))
}

pub fn transition_cost(_start: &Vec3<i32>, _end: &Vec3<i32>) -> f32 {
    1.0f32
}
