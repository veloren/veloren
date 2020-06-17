pub mod fly;
pub mod idle;
pub mod run;

// Reexports
pub use self::{fly::FlyAnimation, idle::IdleAnimation, run::RunAnimation};

use super::{Bone, FigureBoneData, Skeleton};
use common::comp::{self};
use vek::Vec3;

#[derive(Clone, Default)]
pub struct BirdMediumSkeleton {
    head: Bone,
    torso: Bone,
    tail: Bone,
    wing_l: Bone,
    wing_r: Bone,
    leg_l: Bone,
    leg_r: Bone,
}

impl BirdMediumSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for BirdMediumSkeleton {
    type Attr = SkeletonAttr;

    fn bone_count(&self) -> usize { 7 }

    fn compute_matrices(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let torso_mat = self.torso.compute_base_matrix();

        (
            [
                FigureBoneData::new(torso_mat * self.head.compute_base_matrix()),
                FigureBoneData::new(torso_mat),
                FigureBoneData::new(torso_mat * self.tail.compute_base_matrix()),
                FigureBoneData::new(torso_mat * self.wing_l.compute_base_matrix()),
                FigureBoneData::new(torso_mat * self.wing_r.compute_base_matrix()),
                FigureBoneData::new(self.leg_l.compute_base_matrix()),
                FigureBoneData::new(self.leg_r.compute_base_matrix()),
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
        self.torso.interpolate(&target.torso, dt);
        self.tail.interpolate(&target.tail, dt);
        self.wing_l.interpolate(&target.wing_l, dt);
        self.wing_r.interpolate(&target.wing_r, dt);
        self.leg_l.interpolate(&target.leg_l, dt);
        self.leg_r.interpolate(&target.leg_r, dt);
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    chest: (f32, f32),
    tail: (f32, f32),
    wing: (f32, f32, f32),
    foot: (f32, f32, f32),
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::BirdMedium(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            tail: (0.0, 0.0),
            wing: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a comp::bird_medium::Body> for SkeletonAttr {
    fn from(body: &'a comp::bird_medium::Body) -> Self {
        use comp::bird_medium::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Duck, _) => (4.0, 3.0),
                (Chicken, _) => (4.0, 3.0),
                (Goose, _) => (5.0, 5.0),
                (Peacock, _) => (4.0, 7.0),
                (Eagle, _) => (2.5, 5.0),
                (Snowyowl, _) => (2.5, 5.0),
                (Parrot, _) => (0.5, 4.5),
                (Cockatrice, _) => (3.0, 4.0),
            },
            chest: match (body.species, body.body_type) {
                (Duck, _) => (0.0, 5.0),
                (Chicken, _) => (0.0, 5.0),
                (Goose, _) => (0.0, 8.0),
                (Peacock, _) => (0.0, 10.0),
                (Eagle, _) => (0.0, 8.0),
                (Snowyowl, _) => (0.0, 4.5),
                (Parrot, _) => (0.0, 5.0),
                (Cockatrice, _) => (0.0, 12.5),
            },
            tail: match (body.species, body.body_type) {
                (Duck, _) => (-3.0, 1.5),
                (Chicken, _) => (-3.0, 1.5),
                (Goose, _) => (-5.0, 3.0),
                (Peacock, _) => (-5.5, 2.0),
                (Eagle, _) => (-8.0, -4.0),
                (Snowyowl, _) => (-6.0, -2.0),
                (Parrot, _) => (-8.0, -2.0),
                (Cockatrice, _) => (-10.0, -2.5),
            },
            wing: match (body.species, body.body_type) {
                (Duck, _) => (2.75, 0.0, 1.0),
                (Chicken, _) => (2.75, 0.0, 1.0),
                (Goose, _) => (3.75, -1.0, 2.0),
                (Peacock, _) => (3.0, 0.0, 1.0),
                (Eagle, _) => (3.0, -8.0, 4.0),
                (Snowyowl, _) => (3.5, -5.5, 4.0),
                (Parrot, _) => (2.0, -4.5, 3.0),
                (Cockatrice, _) => (4.5, -2.5, 1.5),
            },
            foot: match (body.species, body.body_type) {
                (Duck, _) => (2.0, -1.5, 4.0),
                (Chicken, _) => (2.0, -1.5, 4.0),
                (Goose, _) => (2.0, -1.5, 7.0),
                (Peacock, _) => (2.0, -2.5, 8.0),
                (Eagle, _) => (2.0, -2.0, 8.0),
                (Snowyowl, _) => (1.5, -2.5, 7.0),
                (Parrot, _) => (1.5, -3.0, 3.0),
                (Cockatrice, _) => (4.0, -3.5, 12.0),
            },
        }
    }
}
