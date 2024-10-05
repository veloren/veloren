use crate::{make_bone, Offsets};

use super::{
    vek::{Lerp, Mat4, Transform, Vec3, Vec4},
    Skeleton,
};
use common::comp;
use vek::quaternion::repr_simd::Quaternion;

pub type Body = comp::plugin::Body;

skeleton_impls!(struct PluginSkeleton {
    + bone0,
    + bone1,
    + bone2,
    + bone3,
    + bone4,
    + bone5,
    + bone6,
    + bone7,
    + bone8,
    + bone9,
    + bone10,
    + bone11,
    + bone12,
    + bone13,
    + bone14,
    + bone15,
});

impl PluginSkeleton {
    pub fn from_module(skel: common_state::plugin::module::Skeleton) -> Self {
        fn convert(
            a: Option<&common_state::plugin::module::Transform>,
        ) -> Transform<f32, f32, f32> {
            a.map_or_else(
                || Transform {
                    position: Vec3::zero(),
                    orientation: Quaternion::identity(),
                    scale: Vec3::one(),
                },
                |a| Transform {
                    position: Vec3::from(a.position),
                    orientation: Quaternion::from_vec4(Vec4::from(a.orientation)),
                    scale: Vec3::from(a.scale),
                },
            )
        }
        Self {
            bone0: convert(skel.first()),
            bone1: convert(skel.get(1)),
            bone2: convert(skel.get(2)),
            bone3: convert(skel.get(3)),
            bone4: convert(skel.get(4)),
            bone5: convert(skel.get(5)),
            bone6: convert(skel.get(6)),
            bone7: convert(skel.get(7)),
            bone8: convert(skel.get(8)),
            bone9: convert(skel.get(9)),
            bone10: convert(skel.get(10)),
            bone11: convert(skel.get(11)),
            bone12: convert(skel.get(12)),
            bone13: convert(skel.get(13)),
            bone14: convert(skel.get(14)),
            bone15: convert(skel.get(15)),
        }
    }
}

impl Skeleton for PluginSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 16;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"plugin_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "plugin_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [crate::FigureBoneData; crate::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> crate::Offsets {
        let base_mat = base_mat * Mat4::scaling_3d(1.0 / 13.0);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(base_mat * Mat4::<f32>::from(self.bone0)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone1)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone2)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone3)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone4)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone5)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone6)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone7)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone8)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone9)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone10)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone11)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone12)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone13)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone14)),
            make_bone(base_mat * Mat4::<f32>::from(self.bone15)),
        ];
        Offsets {
            lantern: None,
            viewpoint: Some(
                (base_mat * Mat4::<f32>::from(self.bone0) * Vec4::new(0.0, 3.0, 0.0, 1.0)).xyz(),
            ),
            // TODO: see quadruped_medium for how to animate this
            mount_bone: Transform {
                position: comp::Body::Plugin(body).mount_offset().into_tuple().into(),
                ..Default::default()
            },
            primary_trail_mat: None,
            secondary_trail_mat: None,
            heads: Default::default(),
            tail: None,
        }
    }
}

#[derive(Default)]
pub struct SkeletonAttr {}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Plugin(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(_body: &'a Body) -> Self { Self {} }
}
