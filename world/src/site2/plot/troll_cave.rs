use super::*;
use crate::{
    assets::AssetHandle,
    site2::gen::PrimitiveTransform,
    util::{sampler::Sampler, RandomField},
    Land,
};
use common::{
    generation::EntityInfo,
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use vek::*;

pub struct TrollCave {
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
    temp: f32,
}
impl TrollCave {
    pub fn generate(
        land: &Land,
        _rng: &mut impl Rng,
        site: &Site,
        tile_aabr: Aabr<i32>,
        site_temp: f32,
    ) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        let temp = site_temp;
        Self {
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos(tile_aabr.center())) as i32 + 2,
            temp,
        }
    }
}

impl Structure for TrollCave {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_troll_cave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_troll_cave")]
    fn render_inner(&self, _site: &Site, land: &Land, painter: &Painter) {
        let center = self.bounds.center();
        let base = land.get_alt_approx(center) as i32;
        let mut thread_rng = thread_rng();
        let model_pos = center.with_z(base);
        // model
        lazy_static! {
            pub static ref MODEL: AssetHandle<StructuresGroup> =
                PrefabStructure::load_group("site_structures.troll_cave.troll_cave");
        }
        let rng = RandomField::new(0).get(model_pos) % 10;
        let model = MODEL.read();
        let model = model[rng as usize % model.len()].clone();
        painter
            .prim(Primitive::Prefab(Box::new(model.clone())))
            .translate(model_pos)
            .fill(Fill::Prefab(Box::new(model), model_pos, rng));
        let temp = self.temp;
        // npcs
        let troll = if temp >= CONFIG.tropical_temp {
            "common.entity.wild.aggressive.swamp_troll"
        } else if temp <= (CONFIG.snow_temp) {
            "common.entity.wild.aggressive.mountain_troll"
        } else {
            "common.entity.wild.aggressive.cave_troll"
        };

        // troll
        painter.spawn(
            EntityInfo::at(center.with_z(base - 15).as_()).with_asset_expect(
                troll,
                &mut thread_rng,
                None,
            ),
        );
        // bat
        painter.spawn(
            EntityInfo::at((center - 2).with_z(base + 5).as_()).with_asset_expect(
                "common.entity.wild.peaceful.bat",
                &mut thread_rng,
                None,
            ),
        )
    }
}
