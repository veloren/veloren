pub mod idle;

// Reexports
pub use self::idle::IdleAnimation;

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::ship::Body;

skeleton_impls!(struct ShipSkeleton {
    + bone0,
    + bone1,
    + bone2,
    + bone3,
});

impl Skeleton for ShipSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 4;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"ship_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "ship_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let scale_mat = Mat4::scaling_3d(1.0 / 11.0);

        let bone0_mat = base_mat * scale_mat * Mat4::<f32>::from(self.bone0);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(bone0_mat),
            make_bone(bone0_mat * Mat4::<f32>::from(self.bone1)),
            make_bone(bone0_mat * Mat4::<f32>::from(self.bone2)),
            make_bone(bone0_mat * Mat4::<f32>::from(self.bone3)),
        ];
        Offsets {
            lantern: None,
            viewpoint: None,
            // TODO: see quadruped_medium for how to animate this
            mount_bone: Transform {
                position: (base_mat * scale_mat)
                    .mul_point(comp::Body::Ship(body).mount_offset().into_tuple().into()),
                ..Default::default()
            },
            primary_trail_mat: None,
            secondary_trail_mat: None,
        }
    }
}

pub struct SkeletonAttr {
    bone0: (f32, f32, f32),
    bone1: (f32, f32, f32),
    bone2: (f32, f32, f32),
    bone3: (f32, f32, f32),
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Ship(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            bone0: (0.0, 0.0, 0.0),
            bone1: (0.0, 0.0, 0.0),
            bone2: (0.0, 0.0, 0.0),
            bone3: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::ship::Body::*;
        Self {
            bone0: match body {
                DefaultAirship => (0.0, 0.0, 0.0),
                AirBalloon => (0.0, 0.0, 0.0),
                SailBoat => (0.0, 0.0, 0.0),
                Galleon => (0.0, 0.0, 0.0),
                Volume => (0.0, 0.0, 0.0),
            },
            bone1: match body {
                DefaultAirship => (-13.0, -25.0, 10.0),
                AirBalloon => (0.0, 0.0, 0.0),
                SailBoat => (0.0, 0.0, 0.0),
                Galleon => (0.0, 0.0, 0.0),
                Volume => (0.0, 0.0, 0.0),
            },
            bone2: match body {
                DefaultAirship => (13.0, -25.0, 10.0),
                AirBalloon => (0.0, 0.0, 0.0),
                SailBoat => (0.0, 0.0, 0.0),
                Galleon => (0.0, 0.0, 0.0),
                Volume => (0.0, 0.0, 0.0),
            },
            bone3: match body {
                DefaultAirship => (0.0, -27.5, 8.5),
                AirBalloon => (0.0, -9.0, 8.0),
                SailBoat => (0.0, 0.0, 0.0),
                Galleon => (0.0, 0.0, 0.0),
                Volume => (0.0, 0.0, 0.0),
            },
        }
    }
}
