pub mod idle;

// Reexports
pub use self::idle::IdleAnimation;

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::comp::{self, body::item::ItemArmorKind};
use core::convert::TryFrom;
use std::f32::consts::PI;

pub type Body = comp::body::item::Body;

skeleton_impls!(struct ItemSkeleton {
    + bone0,
});

impl Skeleton for ItemSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 1;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"item_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "item_compute_mats"))]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let scale_mat = Mat4::scaling_3d(1.0 / 11.0 * Self::scale(&body));

        let bone0_mat = base_mat * scale_mat * Mat4::<f32>::from(self.bone0);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) =
            [make_bone(bone0_mat)];
        Offsets {
            lantern: Some((bone0_mat * Vec4::new(0.0, 0.0, 3.5, 1.0)).xyz()),
            mount_bone: Transform {
                position: comp::Body::Item(body).mount_offset().into_tuple().into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl ItemSkeleton {
    pub fn scale(body: &Body) -> f32 {
        match body {
            Body::Tool(_) | Body::Thrown(_) => 0.8,
            Body::Glider => 0.45,
            Body::Coins => 0.3,
            Body::Armor(kind) => match kind {
                ItemArmorKind::Neck | ItemArmorKind::Ring => 0.5,
                ItemArmorKind::Back => 0.7,
                _ => 0.8,
            },
            _ => 1.0,
        }
    }
}

pub struct SkeletonAttr {
    bone0: (f32, f32, f32, f32),
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Item(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            bone0: (0.0, 0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        match body {
            Body::Thrown(_) => Self {
                bone0: (0.0, 0.0, 0.0, -PI / 2.0),
            },
            _ => Self {
                bone0: (0.0, 0.0, 0.0, 0.0),
            },
        }
    }
}
