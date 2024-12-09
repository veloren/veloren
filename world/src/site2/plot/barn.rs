use super::*;
use crate::{site2::gen::PrimitiveTransform, Land};
use common::terrain::Structure as PrefabStructure;
use rand::prelude::*;
use vek::*;
use crate::util::{RandomField, Sampler};
use common::generation::EntityInfo;

pub struct Barn {
    /// Tile position of the door tile
    pub door_tile: Vec2<i32>,
    /// Axis aligned bounding region for the house
    bounds: Aabr<i32>,
    /// Approximate altitude of the door tile
    pub(crate) alt: i32,
}

impl Barn {
    pub fn generate(
        land: &Land,
        _rng: &mut impl Rng,
        site: &Site,
        door_tile: Vec2<i32>,
        door_dir: Vec2<i32>,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        let door_tile_pos = site.tile_center_wpos(door_tile);
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        Self {
            door_tile: door_tile_pos,
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos(door_tile + door_dir)) as i32 + 2,
        }
    }
}

impl Structure for Barn {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_barn\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_barn")]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let base = self.alt;
        let center = self.bounds.center();
        
        let length = (22 + RandomField::new(0).get(center.with_z(base)) % 3) as i32;
        let width = (8 + RandomField::new(0).get((center - 1).with_z(base)) % 3) as i32;

        // solid dirt
        let fill = Fill::Block(Block::new(BlockKind::Earth, Rgb::new(161, 116, 86)));

        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length - 6, center.y - width - 6).with_z(base - 10),
                max: Vec2::new(center.x + length + 7, center.y + width + 7).with_z(base),
            })
            .fill(fill);

        // Barn prefab
        let barn_path = "site_structures.plot_structures.barn";
        let entrance_pos: Vec3<i32> = (center.x, center.y, self.alt).into();

        let barn_site_pos: Vec3<i32> = entrance_pos + Vec3::new(0, 0, 0);
        render_prefab(barn_path, barn_site_pos, painter);

        // barn animals
        let mut thread_rng = thread_rng();

        let barn_animals = [
            "common.entity.wild.peaceful.cattle",
            "common.entity.wild.peaceful.horse",
        ];
        
        for _ in 1..=5 {
            let npc_rng = thread_rng.gen_range(0..=1);
            let spec = barn_animals[npc_rng];

            painter.spawn(
                EntityInfo::at(Vec3::new(center.x, center.y, self.alt).as_()).with_asset_expect(
                    spec,
                    &mut thread_rng,
                    None,
                ));
        }
    }
}

fn render_prefab(file_path: &str, position: Vec3<i32>, painter: &Painter) {
    let asset_handle = PrefabStructure::load_group(file_path);
    let prefab_structure = asset_handle.read()[0].clone();

    // Render the prefab
    painter
        .prim(Primitive::Prefab(Box::new(prefab_structure.clone())))
        .translate(position)
        .fill(Fill::Prefab(Box::new(prefab_structure), position, 0));
}
