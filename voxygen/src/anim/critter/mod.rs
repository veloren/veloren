pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;
use common::comp::{self};
use vek::Vec3;

#[derive(Clone, Default)]
pub struct CritterSkeleton {
    head: Bone,
    chest: Bone,
    feet_f: Bone,
    feet_b: Bone,
    tail: Bone,
}
pub struct CritterAttr {
    head: (f32, f32),
    chest: (f32, f32),
    feet_f: (f32, f32),
    feet_b: (f32, f32),
    tail: (f32, f32),
}

impl CritterSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for CritterSkeleton {
    type Attr = CritterAttr;

    fn bone_count(&self) -> usize { 5 }

    fn compute_matrices(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        (
            [
                FigureBoneData::new(self.head.compute_base_matrix()),
                FigureBoneData::new(self.chest.compute_base_matrix()),
                FigureBoneData::new(self.feet_f.compute_base_matrix()),
                FigureBoneData::new(self.feet_b.compute_base_matrix()),
                FigureBoneData::new(self.tail.compute_base_matrix()),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
            ],
            Vec3::default(),
        )
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.chest.interpolate(&target.chest, dt);
        self.feet_f.interpolate(&target.feet_f, dt);
        self.feet_b.interpolate(&target.feet_b, dt);
        self.tail.interpolate(&target.tail, dt);
    }
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for CritterAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Critter(body) => Ok(CritterAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl CritterAttr {
    pub fn calculate_scale(body: &comp::critter::Body) -> f32 {
        match (body.species, body.body_type) {
            (_, _) => 0.0,
        }
    }
}

impl Default for CritterAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            feet_f: (0.0, 0.0),
            feet_b: (0.0, 0.0),
            tail: (0.0, 0.0),
        }
    }
}

impl<'a> From<&'a comp::critter::Body> for CritterAttr {
    fn from(body: &'a comp::critter::Body) -> Self {
        use comp::critter::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Rat, _) => (6.5, 7.0),
                (Axolotl, _) => (5.0, 5.0),
                (Gecko, _) => (5.0, 4.0),
                (Turtle, _) => (8.0, 7.0),
                (Squirrel, _) => (5.0, 4.0),
                (Fungome, _) => (4.0, 4.0),
            },
            chest: match (body.species, body.body_type) {
                (Rat, _) => (0.0, 6.0),
                (Axolotl, _) => (-1.0, 3.0),
                (Gecko, _) => (-2.0, 3.0),
                (Turtle, _) => (0.0, 6.0),
                (Squirrel, _) => (0.0, 3.0),
                (Fungome, _) => (0.0, 5.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Rat, _) => (2.0, 0.5),
                (Axolotl, _) => (2.0, 0.5),
                (Gecko, _) => (1.0, 0.5),
                (Turtle, _) => (3.0, 0.5),
                (Squirrel, _) => (1.0, 0.5),
                (Fungome, _) => (1.0, 0.5),
            },
            feet_b: match (body.species, body.body_type) {
                (Rat, _) => (-2.0, 0.5),
                (Axolotl, _) => (-2.0, 0.5),
                (Gecko, _) => (-2.0, 0.5),
                (Turtle, _) => (-2.0, 0.5),
                (Squirrel, _) => (-1.0, 0.5),
                (Fungome, _) => (-2.0, 0.5),
            },
            tail: match (body.species, body.body_type) {
                (Rat, _) => (-8.0, 3.0),
                (Axolotl, _) => (-7.0, 3.0),
                (Gecko, _) => (-7.0, 2.0),
                (Turtle, _) => (-6.0, 4.0),
                (Squirrel, _) => (-3.0, 4.0),
                (Fungome, _) => (-6.0, 3.0),
            },
        }
    }
}
