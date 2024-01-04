use crate::{
    layer::tree::{ProceduralTree, TreeConfig},
    site::namegen::NameGen,
    site2::{Fill, Painter, Site, Structure},
    util::FastNoise,
    Land, Sampler, SpriteKind,
};
use common::{
    generation::EntityInfo,
    terrain::{Block, BlockKind},
};
use rand::Rng;
use vek::*;

pub struct GiantTree {
    name: String,
    wpos: Vec3<i32>,
    tree: ProceduralTree,
    seed: u32,
}

impl GiantTree {
    pub fn generate(site: &Site, center_tile: Vec2<i32>, land: &Land, rng: &mut impl Rng) -> Self {
        let wpos = site.tile_center_wpos(center_tile);
        Self {
            name: format!("Tree of {}", NameGen::location(rng).generate()),
            // Get the tree's altitude
            wpos: wpos.with_z(land.get_alt_approx(wpos) as i32),
            tree: {
                let config = TreeConfig::giant(rng, 4.0, true);
                ProceduralTree::generate(config, rng)
            },
            seed: rng.gen(),
        }
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn radius(&self) -> f32 { 100.0 }

    pub fn tree(&self) -> &ProceduralTree { &self.tree }

    pub fn entity_at(
        &self,
        pos: Vec3<i32>,
        above_block: &Block,
        dynamic_rng: &mut impl Rng,
    ) -> Option<EntityInfo> {
        if above_block.kind() == BlockKind::Leaves && dynamic_rng.gen_bool(0.001) {
            let entity = EntityInfo::at(pos.as_());
            match dynamic_rng.gen_range(0..=4) {
                0 => Some(entity.with_asset_expect(
                    "common.entity.wild.aggressive.horn_beetle",
                    dynamic_rng,
                    None,
                )),
                1 => Some(entity.with_asset_expect(
                    "common.entity.wild.aggressive.stag_beetle",
                    dynamic_rng,
                    None,
                )),
                2 => Some(entity.with_asset_expect(
                    "common.entity.wild.aggressive.deadwood",
                    dynamic_rng,
                    None,
                )),
                3 => Some(entity.with_asset_expect(
                    "common.entity.wild.aggressive.maneater",
                    dynamic_rng,
                    None,
                )),
                4 => Some(entity.with_asset_expect(
                    "common.entity.wild.peaceful.parrot",
                    dynamic_rng,
                    None,
                )),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl Structure for GiantTree {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_gianttree\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_gianttree")]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let fast_noise = FastNoise::new(self.seed);
        let dark = Rgb::new(10, 70, 50).map(|e| e as f32);
        let light = Rgb::new(80, 140, 10).map(|e| e as f32);
        let leaf_col = Lerp::lerp(
            dark,
            light,
            fast_noise.get((self.wpos.map(|e| e as f64) * 0.05) * 0.5 + 0.5),
        );
        let mut rng = rand::thread_rng();
        self.tree.walk(|branch, parent| {
            let aabr = Aabr {
                min: self.wpos.xy() + branch.get_aabb().min.xy().as_(),
                max: self.wpos.xy() + branch.get_aabb().max.xy().as_(),
            };
            if aabr.collides_with_aabr(painter.render_aabr().as_()) {
                painter
                    .line_two_radius(
                        self.wpos + branch.get_line().start.as_(),
                        self.wpos + branch.get_line().end.as_(),
                        parent.get_wood_radius(),
                        branch.get_wood_radius(),
                    )
                    .fill(Fill::Block(Block::new(
                        BlockKind::Wood,
                        Rgb::new(80, 32, 0),
                    )));
                if branch.get_leaf_radius() > branch.get_wood_radius() {
                    painter
                        .line_two_radius(
                            self.wpos + branch.get_line().start.as_(),
                            self.wpos + branch.get_line().end.as_(),
                            parent.get_leaf_radius(),
                            branch.get_leaf_radius(),
                        )
                        .fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            leaf_col.map(|e| e as u8),
                        )));
                    // Calculate direction of the branch
                    let branch_start = branch.get_line().start;
                    let branch_end = branch.get_line().end;
                    let branch_direction = (branch_end - branch_start).normalized();

                    // Generate a sprite at the surface of the leafy part of the branch
                    let displacement =
                        (branch_direction * branch.get_leaf_radius()).map(|e| e.round() as i32);
                    let pos = self.wpos + branch_end.as_() + displacement;
                    if rng.gen_bool(0.07) {
                        let ori = rng.gen_range(0..=3) * 2;
                        painter.resource_sprite(pos, SpriteKind::Ironwood, ori);
                    }
                }
                true
            } else {
                false
            }
        });
    }
}
