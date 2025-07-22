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
use std::f32::consts::TAU;
use vek::*;

pub struct PirateHideout {
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
}
impl PirateHideout {
    pub fn generate(land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        Self {
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos((tile_aabr.max - tile_aabr.min) / 2))
                as i32
                + 2,
        }
    }
}

impl Structure for PirateHideout {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_pirate_hideout\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "render_pirate_hideout"))]
    fn render_inner(&self, _site: &Site, land: &Land, painter: &Painter) {
        let center = self.bounds.center();
        let base = land.get_alt_approx(center) as i32;
        let mut thread_rng = thread_rng();
        let model_pos = center.with_z(base - 2);
        // model
        lazy_static! {
            pub static ref MODEL: AssetHandle<StructuresGroup> =
                PrefabStructure::load_group("site_structures.pirate_hideout.pirate_hideout");
        }
        let rng = RandomField::new(0).get(model_pos) % 10;
        let model = MODEL.read();
        let model = model[rng as usize % model.len()].clone();
        painter
            .prim(Primitive::Prefab(Box::new(model.clone())))
            .translate(model_pos)
            .fill(Fill::Prefab(Box::new(model), model_pos, rng));

        // npcs
        let npc_radius = 15;
        let phi = TAU / 2.0;
        for n in 1..=2 {
            let npc_pos = Vec2::new(
                center.x + (npc_radius as f32 * ((n as f32 * phi).cos())) as i32,
                center.y + (npc_radius as f32 * ((n as f32 * phi).sin())) as i32,
            );

            match RandomField::new(0).get(npc_pos.with_z(base + 2)) % 2 {
                // rat
                0 => painter.spawn(
                    EntityInfo::at(npc_pos.with_z(base).as_()).with_asset_expect(
                        "common.entity.wild.peaceful.rat",
                        &mut thread_rng,
                        None,
                    ),
                ),
                // parrot
                _ => painter.spawn(
                    EntityInfo::at(npc_pos.with_z(base).as_()).with_asset_expect(
                        "common.entity.wild.peaceful.parrot",
                        &mut thread_rng,
                        None,
                    ),
                ),
            }
        }
    }
}
