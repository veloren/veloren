use super::{
    EcsEntity,
    cache::{FigureKey, TerrainModelEntryFuture},
    load::{BodySpec, ShipBoneMeshes},
};
use common::{assets, comp::ship::figuredata::VoxelCollider};
use std::{convert::TryFrom, sync::Arc};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct VolumeKey {
    pub entity: EcsEntity,
    pub mut_count: usize,
}

impl From<&Self> for VolumeKey {
    fn from(this: &Self) -> Self { *this }
}

impl anim::Skeleton for VolumeKey {
    type Attr = Self;
    type Body = Self;

    const BONE_COUNT: usize = 4;
    #[cfg(feature = "hot-anim")]
    const COMPUTE_FN: &'static [u8] = b"I AM NOT USED\0";

    // Override compute_matrices so that hotloading is not done for this (since it
    // will fail as this isn't part of the hotloaded anim crate)
    fn compute_matrices(
        &self,
        base_mat: anim::vek::Mat4<f32>,
        buf: &mut [anim::FigureBoneData; anim::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> anim::Offsets {
        self.compute_matrices_inner(base_mat, buf, body)
    }

    fn compute_matrices_inner(
        &self,
        base_mat: anim::vek::Mat4<f32>,
        buf: &mut [anim::FigureBoneData; anim::MAX_BONE_COUNT],
        _: Self::Body,
    ) -> anim::Offsets {
        let bone = base_mat;

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            anim::make_bone(bone),
            anim::make_bone(bone),
            anim::make_bone(bone),
            anim::make_bone(bone),
        ];

        anim::Offsets::default()
    }
}

impl BodySpec for VolumeKey {
    type BoneMesh = ShipBoneMeshes;
    type Extra = Arc<VoxelCollider>;
    type Manifests = ();
    type ModelEntryFuture<const N: usize> = TerrainModelEntryFuture<N>;
    type Spec = ();

    fn load_spec() -> Result<Self::Manifests, assets::Error> { Ok(()) }

    fn reload_watcher(_: &Self::Manifests) -> assets::ReloadWatcher {
        assets::ReloadWatcher::default()
    }

    fn bone_meshes(
        _: &FigureKey<Self>,
        _: &Self::Manifests,
        collider: Self::Extra,
    ) -> [Option<Self::BoneMesh>; anim::MAX_BONE_COUNT] {
        [
            Some((
                collider.volume().clone(),
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
