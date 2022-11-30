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
        let scale_mat = anim::vek::Mat4::scaling_3d(1.0 / 11.0);

        let bone = base_mat * scale_mat;

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            anim::make_bone(bone),
            anim::make_bone(bone),
            anim::make_bone(bone),
            anim::make_bone(bone),
        ];

        anim::Offsets {
            lantern: None,
            viewpoint: None,
            mount_bone: anim::vek::Transform::default(),
            primary_trail_mat: None,
            secondary_trail_mat: None,
        }
    }
}

impl BodySpec for VolumeKey {
    type Extra = Arc<VoxelCollider>;
    type Manifests = ();
    type Spec = ();

    fn load_spec() -> Result<Self::Manifests, assets::Error> { Ok(()) }

    fn reload_watcher(_: &Self::Manifests) -> assets::ReloadWatcher {
        assets::ReloadWatcher::default()
    }

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
