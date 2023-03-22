#![allow(incomplete_features)]
#![allow(clippy::single_match)]
#[cfg(all(feature = "be-dyn-lib", feature = "use-dyn-lib"))]
compile_error!("Can't use both \"be-dyn-lib\" and \"use-dyn-lib\" features at once");

macro_rules! skeleton_impls {
    { struct $Skeleton:ident { $( $(+)? $bone:ident ),* $(,)? $(:: $($field:ident : $field_ty:ty),* $(,)? )? } } => {
        #[derive(Clone, Default)]
        pub struct $Skeleton {
            $(
                $bone: $crate::Bone,
            )*
            $($(
                $field : $field_ty,
            )*)?
        }

        impl<'a, Factor> $crate::vek::Lerp<Factor> for &'a $Skeleton
            where
                Factor: Copy,
                $crate::Bone: Lerp<Factor, Output=$crate::Bone>
        {
            type Output = $Skeleton;

            fn lerp_unclamped_precise(from: Self, to: Self, factor: Factor) -> Self::Output {
                Self::Output {
                    $(
                        $bone: Lerp::lerp_unclamped_precise(from.$bone, to.$bone, factor),
                    )*
                    $($(
                        $field : to.$field.clone(),
                    )*)?
                }
            }

            fn lerp_unclamped(from: Self, to: Self, factor: Factor) -> Self::Output {
                Self::Output {
                    $(
                        $bone: Lerp::lerp_unclamped(from.$bone, to.$bone, factor),
                    )*
                    $($(
                        $field : to.$field.clone(),
                    )*)?
                }
            }
        }
    }
}

pub mod arthropod;
pub mod biped_large;
pub mod biped_small;
pub mod bird_large;
pub mod bird_medium;
pub mod character;
pub mod dragon;
pub mod fish_medium;
pub mod fish_small;
pub mod fixture;
pub mod golem;
pub mod item_drop;
pub mod object;
pub mod quadruped_low;
pub mod quadruped_medium;
pub mod quadruped_small;
pub mod ship;
pub mod theropod;
pub mod vek;

use self::vek::*;
use bytemuck::{Pod, Zeroable};
use common::comp::tool::ToolKind;
#[cfg(feature = "use-dyn-lib")]
use {
    common_dynlib::LoadedLib, lazy_static::lazy_static, std::ffi::CStr, std::sync::Arc,
    std::sync::Mutex,
};

type MatRaw = [[f32; 4]; 4];

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct FigureBoneData(pub MatRaw, pub MatRaw);

pub const MAX_BONE_COUNT: usize = 16;

pub fn make_bone(mat: Mat4<f32>) -> FigureBoneData {
    let normal = mat.map_cols(Vec4::normalized);
    FigureBoneData(mat.into_col_arrays(), normal.into_col_arrays())
}

pub type Bone = Transform<f32, f32, f32>;

#[cfg(feature = "use-dyn-lib")]
lazy_static! {
    static ref LIB: Arc<Mutex<Option<LoadedLib>>> =
        common_dynlib::init("veloren-voxygen-anim", "anim");
}

#[cfg(feature = "use-dyn-lib")]
pub fn init() { lazy_static::initialize(&LIB); }

// Offsets that will be returned after computing the skeleton matrices
pub struct Offsets {
    pub lantern: Option<Vec3<f32>>,
    pub viewpoint: Option<Vec3<f32>>,
    pub mount_bone: Transform<f32, f32, f32>,
    pub primary_trail_mat: Option<(Mat4<f32>, TrailSource)>,
    pub secondary_trail_mat: Option<(Mat4<f32>, TrailSource)>,
}

#[derive(Clone, Copy)]
pub enum TrailSource {
    Weapon,
    GliderLeft,
    GliderRight,
}

impl TrailSource {
    pub fn relative_offsets(&self, tool: Option<ToolKind>) -> (Vec4<f32>, Vec4<f32>) {
        // Offsets
        const GLIDER_VERT: f32 = 5.0;
        const GLIDER_HORIZ: f32 = 15.0;
        // Trail width
        const GLIDER_WIDTH: f32 = 1.0;

        match self {
            Self::Weapon => {
                let lengths = match tool {
                    Some(ToolKind::Sword) => (0.0, 29.25),
                    Some(ToolKind::Axe) => (10.0, 19.25),
                    Some(ToolKind::Hammer) => (10.0, 19.25),
                    Some(ToolKind::Staff) => (10.0, 19.25),
                    Some(ToolKind::Sceptre) => (10.0, 19.25),
                    _ => (0.0, 0.0),
                };
                (
                    Vec4::new(0.0, 0.0, lengths.0, 1.0),
                    Vec4::new(0.0, 0.0, lengths.1, 1.0),
                )
            },
            Self::GliderLeft => (
                Vec4::new(GLIDER_HORIZ, 0.0, GLIDER_VERT, 1.0),
                Vec4::new(GLIDER_HORIZ + GLIDER_WIDTH, 0.0, GLIDER_VERT, 1.0),
            ),
            Self::GliderRight => (
                Vec4::new(-GLIDER_HORIZ, 0.0, GLIDER_VERT, 1.0),
                Vec4::new(-(GLIDER_HORIZ + GLIDER_WIDTH), 0.0, GLIDER_VERT, 1.0),
            ),
        }
    }
}

pub trait Skeleton: Send + Sync + 'static {
    type Attr;
    type Body;

    const BONE_COUNT: usize;

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8];

    fn compute_matrices(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        #[cfg(not(feature = "use-dyn-lib"))]
        {
            self.compute_matrices_inner(base_mat, buf, body)
        }
        #[cfg(feature = "use-dyn-lib")]
        {
            let lock = LIB.lock().unwrap();
            let lib = &lock.as_ref().unwrap().lib;

            let compute_fn: common_dynlib::Symbol<
                fn(&Self, Mat4<f32>, &mut [FigureBoneData; MAX_BONE_COUNT], Self::Body) -> Offsets,
            > = unsafe { lib.get(Self::COMPUTE_FN) }.unwrap_or_else(|e| {
                panic!(
                    "Trying to use: {} but had error: {:?}",
                    CStr::from_bytes_with_nul(Self::COMPUTE_FN)
                        .map(CStr::to_str)
                        .unwrap()
                        .unwrap(),
                    e
                )
            });

            compute_fn(self, base_mat, buf, body)
        }
    }

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets;
}

pub fn compute_matrices<S: Skeleton>(
    skeleton: &S,
    base_mat: Mat4<f32>,
    buf: &mut [FigureBoneData; MAX_BONE_COUNT],
    body: S::Body,
) -> Offsets {
    S::compute_matrices(skeleton, base_mat, buf, body)
}

pub trait Animation {
    type Skeleton: Skeleton;
    type Dependency<'a>;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8];

    /// Returns a new skeleton that is generated by the animation.
    fn update_skeleton_inner(
        _skeleton: &Self::Skeleton,
        _dependency: Self::Dependency<'_>,
        _anim_time: f32,
        _rate: &mut f32,
        _skeleton_attr: &<<Self as Animation>::Skeleton as Skeleton>::Attr,
    ) -> Self::Skeleton;

    /// Calls `update_skeleton_inner` either directly or via `libloading` to
    /// generate the new skeleton.
    fn update_skeleton(
        skeleton: &Self::Skeleton,
        dependency: Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        skeleton_attr: &<<Self as Animation>::Skeleton as Skeleton>::Attr,
    ) -> Self::Skeleton {
        #[cfg(not(feature = "use-dyn-lib"))]
        {
            Self::update_skeleton_inner(skeleton, dependency, anim_time, rate, skeleton_attr)
        }
        #[cfg(feature = "use-dyn-lib")]
        {
            let lock = LIB.lock().unwrap();
            let lib = &lock.as_ref().unwrap().lib;

            let update_fn: common_dynlib::Symbol<
                fn(
                    &Self::Skeleton,
                    Self::Dependency<'_>,
                    f32,
                    &mut f32,
                    &<Self::Skeleton as Skeleton>::Attr,
                ) -> Self::Skeleton,
            > = unsafe {
                //let start = std::time::Instant::now();
                // Overhead of 0.5-5 us (could use hashmap to mitigate if this is an issue)
                lib.get(Self::UPDATE_FN)
                //println!("{}", start.elapsed().as_nanos());
            }
            .unwrap_or_else(|e| {
                panic!(
                    "Trying to use: {} but had error: {:?}",
                    CStr::from_bytes_with_nul(Self::UPDATE_FN)
                        .map(CStr::to_str)
                        .unwrap()
                        .unwrap(),
                    e
                )
            });

            update_fn(skeleton, dependency, anim_time, rate, skeleton_attr)
        }
    }
}
