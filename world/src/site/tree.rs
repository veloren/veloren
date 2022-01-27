use crate::{
    layer::tree::{ProceduralTree, TreeConfig},
    site::SpawnRules,
    util::{FastNoise, Sampler},
    Canvas, Land,
};
use common::{
    generation::EntityInfo,
    terrain::{Block, BlockKind, SpriteKind},
};
use rand::prelude::*;
use vek::*;

// Temporary, do trees through the new site system later
pub struct Tree {
    pub origin: Vec2<i32>,
    alt: i32,
    seed: u32,
    tree: ProceduralTree,
}

impl Tree {
    pub fn generate(origin: Vec2<i32>, land: &Land, rng: &mut impl Rng) -> Self {
        Self {
            origin,
            alt: land.get_alt_approx(origin) as i32,
            seed: rng.gen(),
            tree: {
                let config = TreeConfig::giant(rng, 4.0, true);
                ProceduralTree::generate(config, rng)
            },
        }
    }

    pub fn radius(&self) -> f32 { 512.0 }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        let trunk_radius = 48i32;
        SpawnRules {
            trees: wpos.distance_squared(self.origin) > trunk_radius.pow(2),
            ..SpawnRules::default()
        }
    }

    pub fn render(&self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
        let nz = FastNoise::new(self.seed);

        canvas.foreach_col(|canvas, wpos2d, col| {
            let rpos2d = wpos2d - self.origin;
            let bounds = self.tree.get_bounds().map(|e| e as i32);

            if !Aabr::from(bounds).contains_point(rpos2d) {
                // Skip this column
                return;
            }

            let mut above = true;
            let mut last = None;
            for z in (bounds.min.z..bounds.max.z + 1).rev() {
                let wpos = wpos2d.with_z(self.alt + z);
                let rposf = (wpos - self.origin.with_z(self.alt)).map(|e| e as f32 + 0.5);

                let (branch, leaves, platform, air) = self.tree.is_branch_or_leaves_at(rposf);

                if (branch || leaves) && above && col.snow_cover {
                    canvas.set(
                        wpos + Vec3::unit_z(),
                        Block::new(BlockKind::Snow, Rgb::new(255, 255, 255)),
                    );
                }

                let block = if air {
                    Some(Block::empty())
                } else if leaves {
                    if above && dynamic_rng.gen_bool(0.0005) {
                        canvas.spawn(
                            EntityInfo::at(wpos.map(|e| e as f32) + Vec3::unit_z())
                                .with_asset_expect(
                                    match dynamic_rng.gen_range(0..2) {
                                        0 => "common.entity.wild.aggressive.deadwood",
                                        _ => "common.entity.wild.aggressive.maneater",
                                    },
                                    dynamic_rng,
                                ),
                        );
                    } else if above && dynamic_rng.gen_bool(0.0001) {
                        canvas.spawn(
                            EntityInfo::at(wpos.map(|e| e as f32) + Vec3::unit_z())
                                .with_asset_expect(
                                    "common.entity.wild.aggressive.swamp_troll",
                                    dynamic_rng,
                                ),
                        );
                    }

                    let dark = Rgb::new(10, 70, 50).map(|e| e as f32);
                    let light = Rgb::new(80, 140, 10).map(|e| e as f32);
                    let leaf_col = Lerp::lerp(
                        dark,
                        light,
                        nz.get(rposf.map(|e| e as f64) * 0.05) * 0.5 + 0.5,
                    );

                    Some(Block::new(BlockKind::Leaves, leaf_col.map(|e| e as u8)))
                } else if branch {
                    Some(Block::new(BlockKind::Wood, Rgb::new(80, 32, 0)))
                } else if platform {
                    Some(Block::new(BlockKind::Wood, Rgb::new(180, 130, 50)))
                } else {
                    None
                };

                // Chests in trees
                if last.is_none() && block.is_some() && dynamic_rng.gen_bool(0.00025) {
                    canvas.set(wpos + Vec3::unit_z(), Block::air(SpriteKind::Chest));
                }

                if let Some(block) = block {
                    above = false;
                    canvas.set(wpos, block);
                } else if last.map_or(false, |b: Block| {
                    matches!(b.kind(), BlockKind::Leaves | BlockKind::Wood)
                }) && dynamic_rng.gen_bool(0.0005)
                {
                    canvas.set(wpos, Block::air(SpriteKind::Beehive));
                }

                last = block;
            }
        });
    }
}
