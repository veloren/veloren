use super::*;
use crate::{site2::gen::PrimitiveTransform, Land};
use common::terrain::Structure as PrefabStructure;
use common::terrain::{BlockKind, SpriteKind};
use rand::prelude::*;
use vek::*;
use crate::util::{RandomField, Sampler};

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
        let base = self.alt + 1;
        let center = self.bounds.center();
        
        let length = (22 + RandomField::new(0).get(center.with_z(base)) % 3) as i32;
        let width = (8 + RandomField::new(0).get((center - 1).with_z(base)) % 3) as i32;

        let fill = Fill::Block(Block::new(BlockKind::Earth, Rgb::new(251, 251, 227)));

        // solid dirt
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length - 6, center.y - width - 6).with_z(base - 10),
                max: Vec2::new(center.x + length + 7, center.y + width + 7).with_z(base - 1),
            })
            .fill(fill);

        // Barn prefab
        let barn_path = "site_structures.plot_structures.barn";
        let entrance_pos: Vec3<i32> = (center.x, center.y, self.alt).into();

        let barn_site_pos: Vec3<i32> = entrance_pos + Vec3::new(0, 0, 0);
        render_prefab(barn_path, barn_site_pos, painter);

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
