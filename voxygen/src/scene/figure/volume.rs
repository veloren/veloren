use super::{
    cache::FigureKey,
    load::{BodySpec, BoneMeshes},
    EcsEntity,
};
use common::{
    assets,
    comp::ship::figuredata::VoxelCollider,
    figure::{Cell, Segment},
    vol::ReadVol,
};
use std::{convert::TryFrom, sync::Arc};
use vek::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct VolumeKey {
    pub entity: EcsEntity,
    pub mut_count: usize,
}

impl<'a> From<&'a Self> for VolumeKey {
    fn from(this: &Self) -> Self { *this }
}

impl anim::Skeleton for VolumeKey {
    type Attr = Self;
    type Body = Self;

    const BONE_COUNT: usize = 4;

    //#[cfg(feature = "use-dyn-lib")]
    // TODO

    fn compute_matrices_inner(
        &self,
        base_mat: anim::vek::Mat4<f32>,
        buf: &mut [anim::FigureBoneData; anim::MAX_BONE_COUNT],
        _: Self::Body,
    ) -> anim::Offsets {
        let scale_mat = anim::vek::Mat4::scaling_3d(1.0 / 11.0);

        let bone = base_mat * scale_mat; // * anim::vek::Mat4::<f32>::identity();

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            anim::make_bone(bone),
            anim::make_bone(bone),
            anim::make_bone(bone),
            anim::make_bone(bone),
        ];

        anim::Offsets {
            lantern: None,
            mount_bone: anim::vek::Transform::default(),
        }
    }
}

impl BodySpec for VolumeKey {
    type Extra = Arc<VoxelCollider>;
    type Manifests = ();
    type Spec = ();

    fn load_spec() -> Result<Self::Manifests, assets::Error> { Ok(()) }

    fn is_reloaded(_: &mut Self::Manifests) -> bool { false }

    fn bone_meshes(
        _: &FigureKey<Self>,
        _: &Self::Manifests,
        collider: Self::Extra,
    ) -> [Option<BoneMeshes>; anim::MAX_BONE_COUNT] {
        println!("Generating segment...");
        [
            Some((
                Segment::from_fn(collider.volume().sz, (), |pos| {
                    match collider.volume().get(pos).unwrap().get_color() {
                        Some(col) => Cell::new(col, false, false, false),
                        None => Cell::Empty,
                    }
                }),
                -collider.volume().sz.map(|e| e as f32) / 2.0,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    }
}
