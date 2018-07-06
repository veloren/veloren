// Local
use collide::AABB;

// Library
use coord::prelude::*;

pub struct Entity {
    pos: Vec3f,
    move_dir: Vec3f,
    look_dir: Vec2f,
}

impl Entity {
    pub fn new(pos: Vec3f, move_dir: Vec3f, look_dir: Vec2f) -> Entity {
        Entity {
            pos,
            move_dir, //TODO: maybe even a velocity_dir is needed if the player is thrown around by enemys or spells but tries to move in another direction
            look_dir,
        }
    }

    pub fn get_pos(&self) -> Vec3f {
        self.pos
    }

    pub fn get_move_dir(&self) -> Vec3f {
        self.move_dir
    }

    pub fn get_look_dir(&self) -> Vec2f {
        self.look_dir
    }

    pub fn pos_mut<'a>(&'a mut self) -> &'a mut Vec3f {
        &mut self.pos
    }

    pub fn move_dir_mut<'a>(&'a mut self) -> &'a mut Vec3f {
        &mut self.move_dir
    }

    pub fn look_dir_mut<'a>(&'a mut self) -> &'a mut Vec2f {
        &mut self.look_dir
    }

    pub fn get_aabb(&self) -> AABB {
        AABB::new(
            self.pos - vec3!(0.45, 0.45, 0.0),
            self.pos + vec3!(0.45, 0.45, 1.8),
        )
    }
}
