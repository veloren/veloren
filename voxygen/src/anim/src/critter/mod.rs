pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{vek::Vec3, Bone, FigureBoneData, Skeleton};
use common::comp::{self};

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

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"critter_compute_mats\0";

    fn bone_count(&self) -> usize { 5 }

    #[cfg_attr(feature = "be-dyn-lib", export_name = "critter_compute_mats")]

    fn compute_matrices_inner(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let chest_mat = self.chest.compute_base_matrix();
        (
            [
                FigureBoneData::new(chest_mat * self.head.compute_base_matrix()),
                FigureBoneData::new(chest_mat),
                FigureBoneData::new(chest_mat * self.feet_f.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.feet_b.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.tail.compute_base_matrix()),
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
    #[allow(clippy::match_single_binding)] // TODO: Pending review in #587
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
                (Rat, _) => (6.5, 3.0),
                (Axolotl, _) => (5.0, 1.0),
                (Gecko, _) => (5.0, 0.0),
                (Turtle, _) => (8.0, 3.0),
                (Squirrel, _) => (5.0, 0.0),
                (Fungome, _) => (4.0, 0.0),
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
                (Rat, _) => (2.0, -5.0),
                (Axolotl, _) => (2.0, -5.0),
                (Gecko, _) => (1.0, -2.0),
                (Turtle, _) => (3.0, -5.0),
                (Squirrel, _) => (1.0, -2.0),
                (Fungome, _) => (1.0, -4.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Rat, _) => (-2.0, -5.0),
                (Axolotl, _) => (-2.0, -5.0),
                (Gecko, _) => (-2.0, -2.0),
                (Turtle, _) => (-2.0, -5.0),
                (Squirrel, _) => (-1.0, -2.0),
                (Fungome, _) => (-2.0, -4.0),
            },
            tail: match (body.species, body.body_type) {
                (Rat, _) => (-8.0, -1.0),
                (Axolotl, _) => (-7.0, -1.0),
                (Gecko, _) => (-6.5, -2.0),
                (Turtle, _) => (-6.0, 0.0),
                (Squirrel, _) => (-3.0, 0.0),
                (Fungome, _) => (-6.0, -1.0),
            },
        }
    }
}
