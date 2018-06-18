use coord::prelude::*;

pub struct Entity {
    pos: Vec3f,
    ori: f32,
}

impl Entity {
    pub fn new(pos: Vec3f, ori: f32) -> Entity {
        Entity {
            pos,
            ori,
        }
    }

    pub fn pos(&self) -> Vec3f {
        self.pos
    }

    pub fn ori(&self) -> f32 {
        self.ori
    }

    pub fn pos_mut<'a>(&'a mut self) -> &'a mut Vec3f {
        &mut self.pos
    }

    pub fn ori_mut<'a>(&'a mut self) -> &'a mut f32 {
        &mut self.ori
    }
}
