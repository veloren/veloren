pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::critter::Body;

skeleton_impls!(struct CritterSkeleton {
    + head,
    + chest,
    + feet_f,
    + feet_b,
    + tail,
});

pub struct CritterAttr {
    head: (f32, f32),
    chest: (f32, f32),
    feet_f: (f32, f32),
    feet_b: (f32, f32),
    tail: (f32, f32),
}

impl Skeleton for CritterSkeleton {
    type Attr = CritterAttr;
    type Body = Body;

    const BONE_COUNT: usize = 5;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"critter_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "critter_compute_mats")]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(chest_mat * Mat4::<f32>::from(self.head)),
            make_bone(chest_mat),
            make_bone(chest_mat * Mat4::<f32>::from(self.feet_f)),
            make_bone(chest_mat * Mat4::<f32>::from(self.feet_b)),
            make_bone(chest_mat * Mat4::<f32>::from(self.tail)),
        ];
        Vec3::default()
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

impl<'a> From<&'a Body> for CritterAttr {
    fn from(body: &'a Body) -> Self {
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
