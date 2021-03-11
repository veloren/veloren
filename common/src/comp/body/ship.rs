use crate::{
    make_case_elim
};
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
    pub fn manifest_id(&self) -> &'static str {
        match self {
            Body::DefaultAirship => "server.manifests.ship_manifest",
        }
    }
}

/// Duplicate of some of the things defined in `voxygen::scene::figure::load` to avoid having to
/// refactor all of that to `common` for using voxels as collider geometry
pub mod figuredata {
    use crate::{
        assets::{self, AssetExt, AssetHandle, DotVoxAsset, Ron},
        volumes::dyna::Dyna,
    };
    use serde::Deserialize;
    use hashbrown::HashMap;

    #[derive(Deserialize)]
    pub struct VoxSimple(pub String);

    #[derive(Deserialize)]
    pub struct ShipCentralSpec(pub HashMap<super::Body, SidedShipCentralVoxSpec>);

    #[derive(Deserialize)]
    pub struct SidedShipCentralVoxSpec {
        pub bone0: ShipCentralSubSpec,
        pub bone1: ShipCentralSubSpec,
        pub bone2: ShipCentralSubSpec,
    }

    #[derive(Deserialize)]
    pub struct ShipCentralSubSpec {
        pub offset: [f32; 3],
        pub central: VoxSimple,
    }

    /// manual instead of through `make_vox_spec!` so that it can be in `common`
    #[derive(Clone)]
    pub struct ShipSpec {
        pub central: AssetHandle<Ron<ShipCentralSpec>>,
    }

    impl assets::Compound for ShipSpec {
        fn load<S: assets::source::Source>(_: &assets::AssetCache<S>, _: &str) -> Result<Self, assets::Error> {
            Ok(ShipSpec {
                central: AssetExt::load("server.manifests.ship_manifest")?
            })
        }
    }
}
