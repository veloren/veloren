pub mod idle;
pub mod swim;

// Reexports
pub use self::{idle::IdleAnimation, swim::SwimAnimation};

use super::{FigureBoneData, Offsets, Skeleton, make_bone, vek::*};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::fish_medium::Body;

skeleton_impls!(struct FishMediumSkeleton {
    + head,
    + jaw,
    + chest_front,
    + chest_back,
    + tail,
    + fin_l,
    + fin_r,
});

impl Skeleton for FishMediumSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 7;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"fish_medium_compute_mats\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "fish_medium_compute_mats")
    )]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let base_mat = base_mat * Mat4::scaling_3d(1.0 / 11.0);

        let chest_front_mat = base_mat * Mat4::<f32>::from(self.chest_front);
        let chest_back_mat = Mat4::<f32>::from(self.chest_back);
        let head_mat = chest_front_mat * Mat4::<f32>::from(self.head);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(head_mat * Mat4::<f32>::from(self.jaw)),
            make_bone(chest_front_mat),
            make_bone(chest_front_mat * chest_back_mat),
            make_bone(chest_front_mat * chest_back_mat * Mat4::<f32>::from(self.tail)),
            make_bone(chest_front_mat * Mat4::<f32>::from(self.fin_l)),
            make_bone(chest_front_mat * Mat4::<f32>::from(self.fin_r)),
        ];

        let (mount_bone_mat, mount_bone_ori) = (head_mat, self.head.orientation);
        let mount_position = mount_bone_mat.mul_point(mount_point(&body));
        let mount_orientation = mount_bone_ori;

        Offsets {
            viewpoint: Some((head_mat * Vec4::new(0.0, 5.0, 0.0, 1.0)).xyz()),
            // TODO: see quadruped_medium for how to animate this
            mount_bone: Transform {
                position: mount_position,
                orientation: mount_orientation,
                scale: Vec3::one(),
            },
            ..Default::default()
        }
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    jaw: (f32, f32),
    chest_front: (f32, f32),
    chest_back: (f32, f32),
    tail: (f32, f32),
    fin: (f32, f32, f32),
    tempo: f32,
    amplitude: f32,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::FishMedium(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            jaw: (0.0, 0.0),
            chest_front: (0.0, 0.0),
            chest_back: (0.0, 0.0),
            tail: (0.0, 0.0),
            fin: (0.0, 0.0, 0.0),
            tempo: 0.0,
            amplitude: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::fish_medium::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Marlin, _) => (2.0, 1.5),
                (Icepike, _) => (3.0, 1.0),
            },
            jaw: match (body.species, body.body_type) {
                (Marlin, _) => (2.5, -3.0),
                (Icepike, _) => (0.0, 0.0),
            },
            chest_front: match (body.species, body.body_type) {
                (Marlin, _) => (0.0, 2.5),
                (Icepike, _) => (0.0, 2.5),
            },
            chest_back: match (body.species, body.body_type) {
                (Marlin, _) => (-1.0, 1.0),
                (Icepike, _) => (-4.5, 0.0),
            },
            tail: match (body.species, body.body_type) {
                (Marlin, _) => (-7.0, 0.0),
                (Icepike, _) => (-0.5, 1.5),
            },
            fin: match (body.species, body.body_type) {
                (Marlin, _) => (2.5, 1.0, 3.5),
                (Icepike, _) => (3.5, 3.0, 0.0),
            },
            tempo: match (body.species, body.body_type) {
                (Marlin, _) => 4.0,
                (Icepike, _) => 4.0,
            },
            amplitude: match (body.species, body.body_type) {
                (Marlin, _) => 4.0,
                (Icepike, _) => 4.0,
            },
        }
    }
}

fn mount_point(body: &Body) -> Vec3<f32> {
    use comp::fish_medium::Species::*;
    match (body.species, body.body_type) {
        (Marlin, _) => (0.0, 0.5, 3.0),
        (Icepike, _) => (0.0, 0.5, 4.0),
    }
    .into()
}
