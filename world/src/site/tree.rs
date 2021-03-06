use crate::{
    layer::tree::{ProceduralTree, TreeConfig},
    site::SpawnRules,
    util::{FastNoise, Sampler},
    Canvas, Land,
};
use common::terrain::{Block, BlockKind};
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
                let config = TreeConfig::giant(rng, 4.0, false);
                ProceduralTree::generate(config, rng)
            },
        }
    }

    pub fn radius(&self) -> f32 { 512.0 }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        let trunk_radius = 48i32;
        SpawnRules {
            trees: wpos.distance_squared(self.origin) > trunk_radius.pow(2),
        }
    }

    pub fn render(&self, canvas: &mut Canvas, _dynamic_rng: &mut impl Rng) {
        let nz = FastNoise::new(self.seed);

        canvas.foreach_col(|canvas, wpos2d, col| {
            let rpos2d = wpos2d - self.origin;
            let bounds = self.tree.get_bounds().map(|e| e as i32);

            if !Aabr::from(bounds).contains_point(rpos2d) {
                // Skip this column
                return;
            }

            let mut above = true;
            for z in (bounds.min.z..bounds.max.z + 1).rev() {
                let wpos = wpos2d.with_z(self.alt + z);
                let rposf = (wpos - self.origin.with_z(self.alt)).map(|e| e as f32 + 0.5);

                let (branch, leaves, _, _) = self.tree.is_branch_or_leaves_at(rposf);

                if branch || leaves {
                    if above && col.snow_cover {
                        canvas.set(
                            wpos + Vec3::unit_z(),
                            Block::new(BlockKind::Snow, Rgb::new(255, 255, 255)),
                        );
                    }

                    if leaves {
                        let dark = Rgb::new(10, 70, 50).map(|e| e as f32);
                        let light = Rgb::new(80, 140, 10).map(|e| e as f32);
                        let leaf_col = Lerp::lerp(
                            dark,
                            light,
                            nz.get(rposf.map(|e| e as f64) * 0.05) * 0.5 + 0.5,
                        );
                        canvas.set(
                            wpos,
                            Block::new(BlockKind::Leaves, leaf_col.map(|e| e as u8)),
                        );
                    } else if branch {
                        canvas.set(wpos, Block::new(BlockKind::Wood, Rgb::new(80, 32, 0)));
                    }

                    above = true;
                }
            }
        });
    }
}
