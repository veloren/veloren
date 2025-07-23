use super::{
    Skeleton,
    vek::{Lerp, Mat4, Transform, Vec3, Vec4},
};
use common::comp;
// use vek::quaternion::repr_simd::Quaternion;
use vek::Quaternion;

pub type Body = comp::plugin::Body;

skeleton_impls!(struct PluginSkeleton ComputedPluginSkeleton {
    + bone0
    + bone1
    + bone2
    + bone3
    + bone4
    + bone5
    + bone6
    + bone7
    + bone8
    + bone9
    + bone10
    + bone11
    + bone12
    + bone13
    + bone14
    + bone15
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
    type ComputedSkeleton = ComputedPluginSkeleton;

    const BONE_COUNT: usize = ComputedPluginSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"plugin_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "plugin_compute_mats"))]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [crate::FigureBoneData; crate::MAX_BONE_COUNT],
        _body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(1.0 / 13.0);

        let computed_skeleton = ComputedPluginSkeleton {
            bone0: base_mat * Mat4::<f32>::from(self.bone0),
            bone1: base_mat * Mat4::<f32>::from(self.bone1),
            bone2: base_mat * Mat4::<f32>::from(self.bone2),
            bone3: base_mat * Mat4::<f32>::from(self.bone3),
            bone4: base_mat * Mat4::<f32>::from(self.bone4),
            bone5: base_mat * Mat4::<f32>::from(self.bone5),
            bone6: base_mat * Mat4::<f32>::from(self.bone6),
            bone7: base_mat * Mat4::<f32>::from(self.bone7),
            bone8: base_mat * Mat4::<f32>::from(self.bone8),
            bone9: base_mat * Mat4::<f32>::from(self.bone9),
            bone10: base_mat * Mat4::<f32>::from(self.bone10),
            bone11: base_mat * Mat4::<f32>::from(self.bone11),
            bone12: base_mat * Mat4::<f32>::from(self.bone12),
            bone13: base_mat * Mat4::<f32>::from(self.bone13),
            bone14: base_mat * Mat4::<f32>::from(self.bone14),
            bone15: base_mat * Mat4::<f32>::from(self.bone15),
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
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
