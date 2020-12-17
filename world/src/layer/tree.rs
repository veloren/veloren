use crate::{
    all::ForestKind,
    block::block_from_structure,
    column::ColumnGen,
    util::{RandomPerm, Sampler, UnitChooser},
    Canvas, CONFIG,
};
use common::{
    assets::AssetHandle,
    terrain::{Block, BlockKind, Structure, StructuresGroup},
    vol::ReadVol,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use std::f32;
use vek::*;

lazy_static! {
    static ref OAKS: AssetHandle<StructuresGroup> = Structure::load_group("oaks");
    static ref OAK_STUMPS: AssetHandle<StructuresGroup> = Structure::load_group("oak_stumps");
    static ref PINES: AssetHandle<StructuresGroup> = Structure::load_group("pines");
    static ref PALMS: AssetHandle<StructuresGroup> = Structure::load_group("palms");
    static ref ACACIAS: AssetHandle<StructuresGroup> = Structure::load_group("acacias");
    static ref BAOBABS: AssetHandle<StructuresGroup> = Structure::load_group("baobabs");
    static ref FRUIT_TREES: AssetHandle<StructuresGroup> = Structure::load_group("fruit_trees");
    static ref BIRCHES: AssetHandle<StructuresGroup> = Structure::load_group("birch");
    static ref MANGROVE_TREES: AssetHandle<StructuresGroup> =
        Structure::load_group("mangrove_trees");
    static ref QUIRKY: AssetHandle<StructuresGroup> = Structure::load_group("quirky");
    static ref QUIRKY_DRY: AssetHandle<StructuresGroup> = Structure::load_group("quirky_dry");
    static ref SWAMP_TREES: AssetHandle<StructuresGroup> = Structure::load_group("swamp_trees");
}

static MODEL_RAND: RandomPerm = RandomPerm::new(0xDB21C052);
static UNIT_CHOOSER: UnitChooser = UnitChooser::new(0x700F4EC7);
static QUIRKY_RAND: RandomPerm = RandomPerm::new(0xA634460F);

#[allow(clippy::if_same_then_else)]
pub fn apply_trees_to(canvas: &mut Canvas) {
    struct Tree {
        pos: Vec3<i32>,
        model: Structure,
        seed: u32,
        units: (Vec2<i32>, Vec2<i32>),
    }

    let mut tree_cache = HashMap::new();

    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let trees = info.land().get_near_trees(wpos2d);

        for (tree_wpos, seed) in trees {
            let tree = if let Some(tree) = tree_cache.entry(tree_wpos).or_insert_with(|| {
                let col = ColumnGen::new(info.land()).get((tree_wpos, info.index()))?;

                let is_quirky = QUIRKY_RAND.chance(seed, 1.0 / 500.0);

                // Ensure that it's valid to place a *thing* here
                if col.alt < col.water_level
                    || col.spawn_rate < 0.9
                    || col.water_dist.map(|d| d < 8.0).unwrap_or(false)
                    || col.path.map(|(d, _, _, _)| d < 12.0).unwrap_or(false)
                {
                    return None;
                }

                // Ensure that it's valid to place a tree here
                if !is_quirky && ((seed.wrapping_mul(13)) & 0xFF) as f32 / 256.0 > col.tree_density
                {
                    return None;
                }

                Some(Tree {
                    pos: Vec3::new(tree_wpos.x, tree_wpos.y, col.alt as i32),
                    model: {
                        let models: AssetHandle<_> = if is_quirky {
                            if col.temp > CONFIG.desert_temp {
                                *QUIRKY_DRY
                            } else {
                                *QUIRKY
                            }
                        } else {
                            match col.forest_kind {
                                ForestKind::Oak if QUIRKY_RAND.chance(seed + 1, 1.0 / 16.0) => {
                                    *OAK_STUMPS
                                },
                                ForestKind::Oak if QUIRKY_RAND.chance(seed + 2, 1.0 / 20.0) => {
                                    *FRUIT_TREES
                                },
                                ForestKind::Palm => *PALMS,
                                ForestKind::Acacia => *ACACIAS,
                                ForestKind::Baobab => *BAOBABS,
                                ForestKind::Oak => *OAKS,
                                ForestKind::Pine => *PINES,
                                ForestKind::Birch => *BIRCHES,
                                ForestKind::Mangrove => *MANGROVE_TREES,
                                ForestKind::Swamp => *SWAMP_TREES,
                            }
                        };

                        let models = models.read();
                        models[(MODEL_RAND.get(seed.wrapping_mul(17)) / 13) as usize % models.len()]
                            .clone()
                    },
                    seed,
                    units: UNIT_CHOOSER.get(seed),
                })
            }) {
                tree
            } else {
                continue;
            };

            let bounds = tree.model.get_bounds();
            let mut is_top = true;
            let mut is_leaf_top = true;
            for z in (bounds.min.z..bounds.max.z).rev() {
                let wpos = Vec3::new(wpos2d.x, wpos2d.y, tree.pos.z + z);
                let model_pos = Vec3::from(
                    (wpos - tree.pos)
                        .xy()
                        .map2(Vec2::new(tree.units.0, tree.units.1), |rpos, unit| {
                            unit * rpos
                        })
                        .sum(),
                ) + Vec3::unit_z() * (wpos.z - tree.pos.z);
                block_from_structure(
                    info.index(),
                    if let Some(block) = tree.model.get(model_pos).ok().copied() {
                        block
                    } else {
                        // If we hit an inaccessible block, we're probably outside the model bounds.
                        // Skip this column.
                        break;
                    },
                    wpos,
                    tree.pos.xy(),
                    tree.seed,
                    col,
                    Block::air,
                )
                .map(|block| {
                    // Add a snow covering to the block above under certain circumstances
                    if col.snow_cover
                        && ((block.kind() == BlockKind::Leaves && is_leaf_top)
                            || (is_top && block.is_filled()))
                    {
                        canvas.set(
                            wpos + Vec3::unit_z(),
                            Block::new(BlockKind::Snow, Rgb::new(210, 210, 255)),
                        );
                    }
                    canvas.set(wpos, block);
                    is_leaf_top = false;
                    is_top = false;
                })
                .unwrap_or_else(|| {
                    is_leaf_top = true;
                });
            }
        }
    });
}
