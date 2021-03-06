use crate::{
    layer::tree::{ProceduralTree, TreeConfig},
    site::SpawnRules,
    Canvas, Land,
};
use common::terrain::{Block, BlockKind};
use rand::prelude::*;
use vek::*;

// Temporary, do trees through the new site system later
pub struct Tree {
    pub origin: Vec2<i32>,
    alt: i32,
    tree: ProceduralTree,
}

impl Tree {
    pub fn generate(origin: Vec2<i32>, land: &Land, rng: &mut impl Rng) -> Self {
        Self {
            origin,
            alt: land.get_alt_approx(origin) as i32,
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

    pub fn render(&self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
        canvas.foreach_col(|canvas, wpos2d, col| {
            let rpos2d = wpos2d - self.origin;
            let bounds = self.tree.get_bounds().map(|e| e as i32);

            if !Aabr::from(bounds).contains_point(rpos2d) {
                // Skip this column
                return;
            }

            for z in (bounds.min.z..bounds.max.z + 1).rev() {
                let wpos = wpos2d.with_z(self.alt + z);
                let rposf = (wpos - self.origin.with_z(self.alt)).map(|e| e as f32 + 0.5);

                let (branch, leaves, _, _) = self.tree.is_branch_or_leaves_at(rposf);
                if leaves {
                    canvas.set(wpos, Block::new(BlockKind::Leaves, Rgb::new(30, 130, 40)));
                } else if branch {
                    canvas.set(wpos, Block::new(BlockKind::Wood, Rgb::new(80, 32, 0)));
                }
            }
        });
    }
}
