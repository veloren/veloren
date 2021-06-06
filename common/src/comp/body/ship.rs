use crate::{
    comp::{Density, Mass},
    consts::AIR_DENSITY,
    make_case_elim,
};
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use vek::Vec3;

pub const ALL_BODIES: [Body; 2] = [Body::DefaultAirship, Body::AirBalloon];

make_case_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Body {
        DefaultAirship = 0,
        AirBalloon = 1,
    }
);

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Ship(body) }
}

impl Body {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        Self::random_with(&mut rng)
    }

    pub fn random_with(rng: &mut impl rand::Rng) -> Self { *(&ALL_BODIES).choose(rng).unwrap() }

    pub fn manifest_entry(&self) -> &'static str {
        match self {
            Body::DefaultAirship => "Human_Airship",
            Body::AirBalloon => "Air_Balloon",
        }
    }

    pub fn dimensions(&self) -> Vec3<f32> {
        match self {
            Body::DefaultAirship => Vec3::new(25.0, 50.0, 40.0),
            Body::AirBalloon => Vec3::new(25.0, 50.0, 40.0),
        }
    }

    fn balloon_vol(&self) -> f32 {
        let spheroid_vol = |equat_d: f32, polar_d: f32| -> f32 {
            (std::f32::consts::PI / 6.0) * equat_d.powi(2) * polar_d
        };
        let dim = self.dimensions();
        spheroid_vol(dim.z, dim.y)
    }

    fn hull_vol(&self) -> f32 {
        // height from bottom of keel to deck
        let deck_height = 10_f32;
        let dim = self.dimensions();
        (std::f32::consts::PI / 6.0) * (deck_height * 1.5).powi(2) * dim.y
    }

    pub fn hull_density(&self) -> Density {
        let oak_density = 600_f32;
        let ratio = 0.1;
        Density(ratio * oak_density + (1.0 - ratio) * AIR_DENSITY)
    }

    pub fn density(&self) -> Density { Density(AIR_DENSITY) }

    pub fn mass(&self) -> Mass { Mass((self.hull_vol() + self.balloon_vol()) * self.density().0) }

    pub fn can_fly(&self) -> bool {
        match self {
            Body::DefaultAirship | Body::AirBalloon => true,
        }
    }
}

/// Terrain is 11.0 scale relative to small-scale voxels,
/// airship scale is multiplied by 11 to reach terrain scale.
pub const AIRSHIP_SCALE: f32 = 11.0;

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
