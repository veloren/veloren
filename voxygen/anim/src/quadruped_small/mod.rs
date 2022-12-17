pub mod alpha;
pub mod feed;
pub mod idle;
pub mod jump;
pub mod run;
pub mod stunned;

// Reexports
pub use self::{
    alpha::AlphaAnimation, feed::FeedAnimation, idle::IdleAnimation, jump::JumpAnimation,
    run::RunAnimation, stunned::StunnedAnimation,
};

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::quadruped_small::Body;

skeleton_impls!(struct QuadrupedSmallSkeleton {
    + head,
    + chest,
    + leg_fl,
    + leg_fr,
    + leg_bl,
    + leg_br,
    + tail,
    mount,
});

impl Skeleton for QuadrupedSmallSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 7;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"quadruped_small_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let chest_mat = base_mat
            * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 11.0)
            * Mat4::<f32>::from(self.chest);
        let head_mat = chest_mat * Mat4::<f32>::from(self.head);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(chest_mat),
            make_bone(chest_mat * Mat4::<f32>::from(self.leg_fl)),
            make_bone(chest_mat * Mat4::<f32>::from(self.leg_fr)),
            make_bone(chest_mat * Mat4::<f32>::from(self.leg_bl)),
            make_bone(chest_mat * Mat4::<f32>::from(self.leg_br)),
            make_bone(chest_mat * Mat4::<f32>::from(self.tail)),
        ];
        let (mount_bone_mat, mount_bone_ori) = (chest_mat, self.chest.orientation);
        let mount_position = (mount_bone_mat * Vec4::from_point(mount_point(&body)))
            .homogenized()
            .xyz();
        let mount_orientation = mount_bone_ori;

        Offsets {
            lantern: None,
            viewpoint: Some((head_mat * Vec4::new(0.0, 3.0, 0.0, 1.0)).xyz()),
            mount_bone: Transform {
                position: mount_position,
                orientation: mount_orientation,
                scale: Vec3::one(),
            },
            primary_trail_mat: None,
            secondary_trail_mat: None,
        }
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    chest: (f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    tail: (f32, f32),
    scaler: f32,
    tempo: f32,
    maximize: f32,
    minimize: f32,
    spring: f32,
    feed: f32,
    lateral: f32,
}
impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedSmall(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            tail: (0.0, 0.0),
            scaler: 0.0,
            tempo: 0.0,
            maximize: 0.0,
            minimize: 0.0,
            spring: 0.0,
            feed: 0.0,
            lateral: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::quadruped_small::{BodyType::*, Species::*};
        Self {
            head: match (body.species, body.body_type) {
                (Pig, _) => (5.0, 2.0),
                (Fox, _) => (4.0, 3.0),
                (Sheep, _) => (4.0, 4.0),
                (Boar, _) => (7.0, 0.0),
                (Jackalope, _) => (3.0, 2.0),
                (Skunk, _) => (5.0, 1.5),
                (Cat, _) => (4.0, 3.0),
                (Batfox, _) => (5.0, 1.0),
                (Raccoon, _) => (5.0, 2.0),
                (Quokka, _) => (6.0, 2.0),
                (Holladon, _) => (7.0, 1.0),
                (Hyena, _) => (7.5, 2.0),
                (Rabbit, _) => (4.0, 3.0),
                (Truffler, _) => (7.5, -9.0),
                (Frog, _) => (4.0, 2.0),
                (Rat, _) => (5.0, -1.0),
                (Axolotl, _) => (3.0, 2.0),
                (Gecko, _) => (4.0, 2.0),
                (Turtle, _) => (5.0, -2.0),
                (Squirrel, _) => (3.5, 1.0),
                (Fungome, _) => (1.5, -1.5),
                (Porcupine, _) => (6.0, 1.0),
                (Beaver, _) => (5.5, 0.0),
                (Hare, Male) => (3.0, 2.0),
                (Hare, Female) => (2.5, 3.0),
                (Dog, _) => (3.0, 4.5),
                (Goat, _) => (3.5, 4.0),
                (Seal, _) => (4.0, 2.5),
            },
            chest: match (body.species, body.body_type) {
                (Pig, _) => (0.0, 6.0),
                (Fox, _) => (0.0, 8.0),
                (Sheep, _) => (2.0, 7.0),
                (Boar, _) => (0.0, 9.5),
                (Jackalope, _) => (-2.0, 6.0),
                (Skunk, _) => (0.0, 6.0),
                (Cat, _) => (0.0, 6.0),
                (Batfox, _) => (-2.0, 6.0),
                (Raccoon, _) => (0.0, 5.5),
                (Quokka, _) => (2.0, 6.5),
                (Holladon, _) => (-2.0, 9.0),
                (Hyena, _) => (-2.0, 9.0),
                (Rabbit, _) => (-2.0, 6.0),
                (Truffler, _) => (-2.0, 16.0),
                (Frog, _) => (-2.0, 4.5),
                (Rat, _) => (6.0, 5.0),
                (Axolotl, _) => (3.0, 5.0),
                (Gecko, _) => (7.5, 4.0),
                (Turtle, _) => (1.0, 6.0),
                (Squirrel, _) => (4.0, 5.0),
                (Fungome, _) => (4.0, 4.0),
                (Porcupine, _) => (2.0, 11.0),
                (Beaver, _) => (2.0, 6.0),
                (Hare, Male) => (-2.0, 7.0),
                (Hare, Female) => (-2.0, 6.0),
                (Dog, _) => (-2.0, 8.5),
                (Goat, _) => (2.0, 7.5),
                (Seal, _) => (-2.0, 4.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Pig, _) => (4.5, 3.5, -1.0),
                (Fox, _) => (3.0, 5.0, -5.5),
                (Sheep, _) => (3.5, 2.0, -2.0),
                (Boar, _) => (3.5, 6.0, -5.5),
                (Jackalope, _) => (3.0, 4.0, -2.0),
                (Skunk, _) => (3.5, 4.0, -1.0),
                (Cat, _) => (2.0, 4.0, -1.0),
                (Batfox, _) => (3.0, 4.0, -0.5),
                (Raccoon, _) => (4.0, 4.0, -0.0),
                (Quokka, _) => (3.0, 4.0, -1.0),
                (Holladon, _) => (5.0, 4.0, -2.5),
                (Hyena, _) => (2.5, 5.0, -4.0),
                (Rabbit, _) => (3.0, 3.0, -3.0),
                (Truffler, _) => (2.5, 5.0, -9.0),
                (Frog, _) => (4.5, 6.5, 0.0),
                (Rat, _) => (5.0, 2.5, -1.0),
                (Axolotl, _) => (2.0, 2.0, -2.0),
                (Gecko, _) => (2.0, 4.0, -0.5),
                (Turtle, _) => (5.0, 4.0, -2.0),
                (Squirrel, _) => (3.5, 3.0, -1.0),
                (Fungome, _) => (3.0, 2.0, -1.0),
                (Porcupine, _) => (4.0, 6.5, -9.0),
                (Beaver, _) => (4.5, 4.5, -4.0),
                (Hare, Male) => (3.0, 1.0, -3.0),
                (Hare, Female) => (3.0, 0.5, -4.0),
                (Dog, _) => (3.5, 3.0, -2.5),
                (Goat, _) => (3.0, 2.5, -3.5),
                (Seal, _) => (6.5, 3.0, -2.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Pig, _) => (3.5, -2.0, 0.0),
                (Fox, _) => (3.0, -3.0, -3.0),
                (Sheep, _) => (3.5, -3.5, -2.0),
                (Boar, _) => (3.0, -3.0, -2.5),
                (Jackalope, _) => (3.5, -2.0, 0.0),
                (Skunk, _) => (3.5, -4.0, -1.5),
                (Cat, _) => (2.0, -3.5, -1.0),
                (Batfox, _) => (3.5, -2.0, -0.5),
                (Raccoon, _) => (4.5, -3.0, 0.5),
                (Quokka, _) => (4.0, -4.0, -1.0),
                (Holladon, _) => (4.0, -2.0, -3.0),
                (Hyena, _) => (3.0, -5.0, -2.5),
                (Rabbit, _) => (3.5, -2.0, -1.0),
                (Truffler, _) => (3.0, -5.0, -9.5),
                (Frog, _) => (5.0, -3.5, 0.0),
                (Rat, _) => (5.0, -2.0, 1.0),
                (Axolotl, _) => (2.0, -3.0, -2.0),
                (Gecko, _) => (1.5, -2.0, -0.5),
                (Turtle, _) => (5.5, -2.5, -2.0),
                (Squirrel, _) => (3.5, -3.0, 0.0),
                (Fungome, _) => (3.0, -3.5, -1.0),
                (Porcupine, _) => (4.5, -1.0, -8.0),
                (Beaver, _) => (4.0, -2.5, -3.0),
                (Hare, Male) => (3.5, -1.0, -2.0),
                (Hare, Female) => (3.5, -3.0, -2.0),
                (Dog, _) => (3.0, -3.5, -2.5),
                (Goat, _) => (3.0, -4.0, -2.0),
                (Seal, _) => (4.5, -6.0, -0.5),
            },
            tail: match (body.species, body.body_type) {
                (Pig, _) => (-4.5, 2.5),
                (Fox, _) => (-4.5, 2.0),
                (Sheep, _) => (-5.0, 0.0),
                (Boar, _) => (-6.0, 0.0),
                (Jackalope, _) => (-4.0, 2.0),
                (Skunk, _) => (-4.0, 0.5),
                (Cat, _) => (-3.5, 2.0),
                (Batfox, _) => (0.0, 5.0),
                (Raccoon, _) => (-4.0, 1.0),
                (Quokka, _) => (-6.0, 1.0),
                (Holladon, _) => (-1.0, 4.0),
                (Hyena, _) => (-7.0, 0.0),
                (Rabbit, _) => (-4.0, -0.0),
                (Truffler, _) => (0.0, 0.0),
                (Frog, _) => (0.0, -0.0),
                (Rat, _) => (-1.0, 2.0),
                (Axolotl, _) => (-4.0, -1.0),
                (Gecko, _) => (-4.0, 0.0),
                (Turtle, _) => (-6.0, -2.0),
                (Squirrel, _) => (-4.0, 0.0),
                (Fungome, _) => (-4.0, -2.0),
                (Porcupine, _) => (-6.0, 1.0),
                (Beaver, _) => (-6.5, -1.0),
                (Hare, Male) => (-4.0, -1.0),
                (Hare, Female) => (-4.0, 2.0),
                (Dog, _) => (-5.0, 0.5),
                (Goat, _) => (-7.0, 0.0),
                (Seal, _) => (-1.0, 4.0),
            },
            scaler: match (body.species, body.body_type) {
                (Pig, _) => 0.72,
                (Fox, _) => 0.72,
                (Boar, _) => 0.95,
                (Jackalope, _) => 0.67,
                (Skunk, _) => 0.72,
                (Cat, _) => 0.67,
                (Batfox, _) => 0.9,
                (Holladon, _) => 1.12,
                (Rabbit, _) => 0.56,
                (Frog, _) => 0.56,
                (Rat, _) => 0.5,
                (Axolotl, _) => 0.5,
                (Gecko, _) => 0.56,
                (Turtle, _) => 0.67,
                (Squirrel, _) => 0.4,
                (Fungome, _) => 0.72,
                (Porcupine, _) => 0.65,
                (Hare, _) => 0.65,
                (Seal, _) => 0.9,
                _ => 0.8,
            },
            tempo: match (body.species, body.body_type) {
                (Boar, _) => 1.1,
                (Cat, _) => 1.1,
                (Quokka, _) => 1.2,
                (Hyena, _) => 1.1,
                (Rabbit, _) => 1.15,
                (Frog, _) => 1.15,
                (Rat, _) => 1.0,
                (Axolotl, _) => 1.2,
                (Gecko, _) => 1.1,
                (Turtle, _) => 3.0,
                (Squirrel, _) => 1.15,
                (Porcupine, _) => 1.2,
                (Beaver, _) => 1.2,
                (Hare, _) => 1.15,
                (Seal, _) => 2.5,
                _ => 1.0,
            },
            maximize: match (body.species, body.body_type) {
                (Fox, _) => 1.3,
                (Sheep, _) => 1.1,
                (Boar, _) => 1.4,
                (Jackalope, _) => 1.2,
                (Hyena, _) => 1.4,
                (Rabbit, _) => 1.3,
                (Frog, _) => 1.3,
                (Axolotl, _) => 0.9,
                (Turtle, _) => 0.8,
                (Fungome, _) => 0.7,
                (Hare, _) => 1.3,
                _ => 1.0,
            },
            minimize: match (body.species, body.body_type) {
                (Pig, _) => 0.6,
                (Fox, _) => 1.3,
                (Sheep, _) => 0.8,
                (Jackalope, _) => 0.8,
                (Skunk, _) => 0.9,
                (Cat, _) => 0.8,
                (Quokka, _) => 0.9,
                (Holladon, _) => 0.7,
                (Hyena, _) => 1.4,
                (Rabbit, _) => 0.8,
                (Frog, _) => 0.8,
                (Turtle, _) => 0.8,
                (Fungome, _) => 0.4,
                (Porcupine, _) => 0.9,
                (Beaver, _) => 0.9,
                (Hare, _) => 0.8,
                (Goat, _) => 0.8,
                (Seal, _) => 0.7,
                _ => 1.0,
            },
            spring: match (body.species, body.body_type) {
                (Sheep, _) => 1.2,
                (Boar, _) => 0.8,
                (Jackalope, _) => 2.2,
                (Cat, _) => 1.4,
                (Batfox, _) => 1.1,
                (Raccoon, _) => 1.1,
                (Quokka, _) => 1.3,
                (Holladon, _) => 0.7,
                (Hyena, _) => 1.4,
                (Rabbit, _) => 2.5,
                (Truffler, _) => 0.8,
                (Frog, _) => 2.5,
                (Axolotl, _) => 0.8,
                (Gecko, _) => 0.6,
                (Turtle, _) => 0.7,
                (Fungome, _) => 0.8,
                (Porcupine, _) => 1.3,
                (Beaver, _) => 1.3,
                (Hare, Male) => 2.2,
                (Hare, Female) => 2.5,
                (Goat, _) => 1.2,
                (Seal, _) => 0.7,
                _ => 1.0,
            },
            feed: match (body.species, body.body_type) {
                (Boar, _) => 0.6,
                (Skunk, _) => 0.8,
                (Batfox, _) => 0.7,
                (Raccoon, _) => 0.8,
                (Rabbit, _) => 1.2,
                (Truffler, _) => 0.6,
                (Frog, _) => 0.7,
                (Axolotl, _) => 0.8,
                (Gecko, _) => 0.8,
                (Turtle, _) => 0.5,
                (Fungome, _) => 0.7,
                (Hare, _) => 1.2,
                _ => 1.0,
            },
            lateral: match (body.species, body.body_type) {
                (Axolotl, _) => 1.0,
                (Gecko, _) => 1.0,
                (Turtle, _) => 1.0,
                (Fungome, _) => 1.0,
                _ => 0.0,
            },
        }
    }
}
fn mount_point(body: &Body) -> Vec3<f32> {
    use comp::quadruped_small::{BodyType::*, Species::*};
    match (body.species, body.body_type) {
        (Pig, _) => (0.0, -2.0, -2.5),
        (Fox, _) => (0.0, -4.0, -3.5),
        (Sheep, _) => (0.0, -4.0, -3.5),
        (Boar, _) => (0.0, -2.0, -3.5),
        (Jackalope, _) => (0.0, -4.0, -3.5),
        (Skunk, _) => (0.0, -4.0, -3.5),
        (Cat, _) => (0.0, -5.0, -4.0),
        (Batfox, _) => (0.0, -4.0, -3.0),
        (Raccoon, _) => (0.0, -4.0, -2.5),
        (Quokka, _) => (0.0, -3.0, -3.5),
        (Holladon, _) => (0.0, -2.0, -2.5),
        (Hyena, _) => (0.0, -4.0, -3.5),
        (Rabbit, _) => (0.0, -4.0, -3.5),
        (Truffler, _) => (0.0, -6.0, 6.5),
        (Frog, _) => (0.0, -4.0, -4.5),
        (Rat, _) => (0.0, -4.0, -4.5),
        (Axolotl, _) => (0.0, -4.0, -4.5),
        (Gecko, _) => (0.0, -4.0, -4.5),
        (Turtle, _) => (0.0, -4.0, -4.5),
        (Squirrel, _) => (0.0, -4.0, -4.5),
        (Fungome, _) => (0.0, -4.0, -4.5),
        (Porcupine, _) => (0.0, -4.0, -3.5),
        (Beaver, _) => (0.0, -2.0, -3.5),
        (Hare, Male) => (0.0, -4.0, -4.5),
        (Hare, Female) => (0.0, -4.0, -4.5),
        (Dog, _) => (0.0, -4.0, -2.5),
        (Goat, _) => (0.0, -4.0, -3.5),
        (Seal, _) => (0.0, -2.0, -2.5),
    }
    .into()
}
