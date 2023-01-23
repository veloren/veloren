pub mod idle;

// Reexports
pub use self::idle::IdleAnimation;

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::comp::{self, item_drop::ItemDropArmorKind};
use core::convert::TryFrom;

pub type Body = comp::item_drop::Body;

skeleton_impls!(struct ItemDropSkeleton {
    + bone0,
});

impl Skeleton for ItemDropSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 1;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"item_drop_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "item_drop_compute_mats")]
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
            viewpoint: None,
            mount_bone: Transform {
                position: comp::Body::ItemDrop(body)
                    .mount_offset()
                    .into_tuple()
                    .into(),
                ..Default::default()
            },
            primary_trail_mat: None,
            secondary_trail_mat: None,
        }
    }
}

impl ItemDropSkeleton {
    pub fn scale(body: &Body) -> f32 {
        match body {
            Body::Tool(_) => 0.8,
            Body::Glider => 0.45,
            Body::Coins => 0.3,
            Body::Armor(kind) => match kind {
                ItemDropArmorKind::Neck | ItemDropArmorKind::Ring => 0.5,
                ItemDropArmorKind::Back => 0.7,
                _ => 0.8,
            },
            _ => 1.0,
        }
    }
}

pub struct SkeletonAttr {
    bone0: (f32, f32, f32),
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::ItemDrop(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            bone0: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(_body: &'a Body) -> Self {
        Self {
            bone0: (0.0, 0.0, 0.0),
        }
    }
}
