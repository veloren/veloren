use crate::make_case_elim;
use serde::{Deserialize, Serialize};

make_case_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Body {
        DefaultAirship = 0,
    }
);

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Ship(body) }
}

impl Body {
    pub fn manifest_entry(&self) -> &'static str {
        match self {
            Body::DefaultAirship => "Human_Airship",
        }
    }
}

/// Terrain is 11.0 scale relative to small-scale voxels, and all figures get
/// multiplied by 0.8 in rendering. For now, have a constant in `comp::Scale`
/// that compensates for both of these, but there might be a more elegant way
/// (e.g. using `Scale(0.8)` for everything else and not having a magic number
/// in figure rendering, and multiplying terrain models by 11.0 in animation).
pub const AIRSHIP_SCALE: f32 = 11.0 / 0.8;

/// Duplicate of some of the things defined in `voxygen::scene::figure::load` to
/// avoid having to refactor all of that to `common` for using voxels as
/// collider geometry
pub mod figuredata {
    use crate::{
        assets::{self, AssetExt, AssetHandle, DotVoxAsset, Ron},
        figure::cell::Cell,
        terrain::{
            block::{Block, BlockKind},
            sprite::SpriteKind,
        },
        volumes::dyna::{ColumnAccess, Dyna},
    };
    use hashbrown::HashMap;
    use lazy_static::lazy_static;
    use serde::Deserialize;
    use vek::Vec3;

    #[derive(Deserialize)]
    pub struct VoxSimple(pub String);

    #[derive(Deserialize)]
    pub struct ShipCentralSpec(pub HashMap<super::Body, SidedShipCentralVoxSpec>);

    #[derive(Deserialize)]
    pub struct SidedShipCentralVoxSpec {
        pub bone0: ShipCentralSubSpec,
        pub bone1: ShipCentralSubSpec,
        pub bone2: ShipCentralSubSpec,
        pub bone3: ShipCentralSubSpec,
    }

    #[derive(Deserialize)]
    pub struct ShipCentralSubSpec {
        pub offset: [f32; 3],
        pub phys_offset: [f32; 3],
        pub central: VoxSimple,
    }

    /// manual instead of through `make_vox_spec!` so that it can be in `common`
    #[derive(Clone)]
    pub struct ShipSpec {
        pub central: AssetHandle<Ron<ShipCentralSpec>>,
        pub colliders: HashMap<String, VoxelCollider>,
    }

    #[derive(Clone)]
    pub struct VoxelCollider {
        pub dyna: Dyna<Block, (), ColumnAccess>,
        pub translation: Vec3<f32>,
    }

    impl assets::Compound for ShipSpec {
        fn load<S: assets::source::Source>(
            cache: &assets::AssetCache<S>,
            _: &str,
        ) -> Result<Self, assets::Error> {
            let manifest: AssetHandle<Ron<ShipCentralSpec>> =
                AssetExt::load("server.manifests.ship_manifest")?;
            let mut colliders = HashMap::new();
            for (_, spec) in (manifest.read().0).0.iter() {
                for bone in [&spec.bone0, &spec.bone1, &spec.bone2].iter() {
                    // TODO: Currently both client and server load models and manifests from
                    // "server.voxel.". In order to support CSG procedural airships, we probably
                    // need to load them in the server and sync them as an ECS resource.
                    let vox =
                        cache.load::<DotVoxAsset>(&["server.voxel.", &bone.central.0].concat())?;
                    let dyna = Dyna::<Cell, (), ColumnAccess>::from_vox(&vox.read().0, false);
                    let dyna = dyna.map_into(|cell| {
                        if let Some(rgb) = cell.get_color() {
                            Block::new(BlockKind::Misc, rgb)
                        } else {
                            Block::air(SpriteKind::Empty)
                        }
                    });
                    let collider = VoxelCollider {
                        dyna,
                        translation: Vec3::from(bone.offset) + Vec3::from(bone.phys_offset),
                    };
                    colliders.insert(bone.central.0.clone(), collider);
                }
            }
            Ok(ShipSpec {
                central: manifest,
                colliders,
            })
        }
    }

    lazy_static! {
        // TODO: Load this from the ECS as a resource, and maybe make it more general than ships
        // (although figuring out how to keep the figure bones in sync with the terrain offsets seems
        // like a hard problem if they're not the same manifest)
        pub static ref VOXEL_COLLIDER_MANIFEST: AssetHandle<ShipSpec> = AssetExt::load_expect("server.manifests.ship_manifest");
    }
}
