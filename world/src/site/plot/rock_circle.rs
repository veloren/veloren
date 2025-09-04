use super::*;
use crate::{
    Land,
    assets::AssetHandle,
    site::gen::PrimitiveTransform,
    util::{RandomField, sampler::Sampler},
};
use common::{
    generation::EntityInfo,
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use vek::*;

pub struct RockCircle {
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
}
impl RockCircle {
    pub fn generate(land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        Self {
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos(tile_aabr.center())) as i32 + 2,
        }
    }
}

impl Structure for RockCircle {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_rock_circle\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "render_rock_circle"))]
    fn render_inner(&self, _site: &Site, land: &Land, painter: &Painter) {
        let center = self.bounds.center();
        let base = land.get_alt_approx(center) as i32;
        let mut rng = rand::rng();
        let model_pos = center.with_z(base);
        // model
        lazy_static! {
            pub static ref MODEL: AssetHandle<StructuresGroup> =
                PrefabStructure::load_group("site_structures.rock_circle.rock_circle");
        }
        let rng_val = RandomField::new(0).get(model_pos) % 10;
        let model = MODEL.read();
        let model = model[rng_val as usize % model.len()].clone();
        painter
            .prim(Primitive::Prefab(Box::new(model.clone())))
            .translate(model_pos)
            .fill(Fill::Prefab(Box::new(model), model_pos, rng_val));

        // npcs
        if rng.random_range(0..=8) < 1 {
            // dullahan
            painter.spawn(
                EntityInfo::at(center.with_z(base + 2).as_()).with_asset_expect(
                    "common.entity.wild.aggressive.dullahan",
                    &mut rng,
                    None,
                ),
            )
        }
    }
}
