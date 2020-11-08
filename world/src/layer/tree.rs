use crate::{
    all::ForestKind,
    block::block_from_structure,
    column::ColumnGen,
    util::{RandomPerm, Sampler, UnitChooser},
    Canvas, CONFIG,
};
use common::{
    terrain::{structure::Structure, Block},
    vol::ReadVol,
};
use lazy_static::lazy_static;
use std::{collections::HashMap, f32, sync::Arc};
use vek::*;

lazy_static! {
    pub static ref OAKS: Vec<Arc<Structure>> = Structure::load_group("oaks");
    pub static ref OAK_STUMPS: Vec<Arc<Structure>> = Structure::load_group("oak_stumps");
    pub static ref PINES: Vec<Arc<Structure>> = Structure::load_group("pines");
    pub static ref PALMS: Vec<Arc<Structure>> = Structure::load_group("palms");
    pub static ref SNOW_PINES: Vec<Arc<Structure>> = Structure::load_group("snow_pines");
    pub static ref ACACIAS: Vec<Arc<Structure>> = Structure::load_group("acacias");
    pub static ref FRUIT_TREES: Vec<Arc<Structure>> = Structure::load_group("fruit_trees");
    pub static ref BIRCHES: Vec<Arc<Structure>> = Structure::load_group("birch");
    pub static ref MANGROVE_TREES: Vec<Arc<Structure>> = Structure::load_group("mangrove_trees");
    pub static ref QUIRKY: Vec<Arc<Structure>> = Structure::load_group("quirky");
    pub static ref QUIRKY_DRY: Vec<Arc<Structure>> = Structure::load_group("quirky_dry");
}

static MODEL_RAND: RandomPerm = RandomPerm::new(0xDB21C052);
static UNIT_CHOOSER: UnitChooser = UnitChooser::new(0x700F4EC7);
static QUIRKY_RAND: RandomPerm = RandomPerm::new(0xA634460F);

pub fn apply_trees_to(canvas: &mut Canvas) {
    struct Tree {
        pos: Vec3<i32>,
        model: Arc<Structure>,
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

                // Ensure that it's valid to place a tree here
                if ((seed.wrapping_mul(13)) & 0xFF) as f32 / 256.0 > col.tree_density
                    || col.alt < col.water_level
                    || col.spawn_rate < 0.5
                    || col.water_dist.map(|d| d < 8.0).unwrap_or(false)
                    || col.path.map(|(d, _, _, _)| d < 12.0).unwrap_or(false)
                {
                    return None;
                }

                Some(Tree {
                    pos: Vec3::new(tree_wpos.x, tree_wpos.y, col.alt as i32),
                    model: {
                        let models: &'static [_] = if QUIRKY_RAND.get(seed) % 512 == 17 {
                            if col.temp > CONFIG.desert_temp {
                                &QUIRKY_DRY
                            } else {
                                &QUIRKY
                            }
                        } else {
                            match col.forest_kind {
                                ForestKind::Palm => &PALMS,
                                ForestKind::Savannah => &ACACIAS,
                                ForestKind::Oak if QUIRKY_RAND.get(seed) % 16 == 7 => &OAK_STUMPS,
                                ForestKind::Oak if QUIRKY_RAND.get(seed) % 19 == 7 => &FRUIT_TREES,
                                ForestKind::Oak if QUIRKY_RAND.get(seed) % 14 == 7 => &BIRCHES,
                                ForestKind::Oak => &OAKS,
                                ForestKind::Pine => &PINES,
                                ForestKind::SnowPine => &SNOW_PINES,
                                ForestKind::Mangrove => &MANGROVE_TREES,
                            }
                        };
                        Arc::clone(
                            &models[(MODEL_RAND.get(seed.wrapping_mul(17)) / 13) as usize
                                % models.len()],
                        )
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
            for z in bounds.min.z..bounds.max.z {
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
                .map(|block| canvas.set(wpos, block));
            }
        }
    });
}
