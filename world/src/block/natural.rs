use super::{BlockGen, StructureInfo, StructureMeta};
use crate::{
    all::ForestKind,
    column::{ColumnGen, ColumnSample},
    util::{RandomPerm, Sampler, SmallCache, UnitChooser},
    CONFIG,
};
use common::{assets, terrain::Structure};
use lazy_static::lazy_static;
use std::sync::Arc;
use std::u32;
use vek::*;

static VOLUME_RAND: RandomPerm = RandomPerm::new(0xDB21C052);
static UNIT_CHOOSER: UnitChooser = UnitChooser::new(0x700F4EC7);
static QUIRKY_RAND: RandomPerm = RandomPerm::new(0xA634460F);

pub fn structure_gen<'a>(
    column_gen: &ColumnGen<'a>,
    column_cache: &mut SmallCache<Option<ColumnSample<'a>>>,
    idx: usize,
    st_pos: Vec2<i32>,
    st_seed: u32,
    structure_samples: &[Option<ColumnSample>; 9],
) -> Option<StructureInfo> {
    let st_sample = &structure_samples[idx].as_ref()?;

    // Assuming it's a tree... figure out when it SHOULDN'T spawn
    let random_seed = (st_seed as f64) / (u32::MAX as f64);
    if (st_sample.tree_density as f64) < random_seed
        || st_sample.alt < st_sample.water_level
        || st_sample.spawn_rate < 0.5
        || !st_sample.spawn_rules.trees
    {
        return None;
    }

    let cliff_height = BlockGen::get_cliff_height(
        column_gen,
        column_cache,
        st_pos.map(|e| e as f32),
        &st_sample.close_cliffs,
        st_sample.cliff_hill,
    );

    let wheight = st_sample.alt.max(cliff_height);
    let st_pos3d = Vec3::new(st_pos.x, st_pos.y, wheight as i32);

    let volumes: &'static [_] = if QUIRKY_RAND.get(st_seed) % 512 == 17 {
        if st_sample.temp > CONFIG.desert_temp {
            &QUIRKY_DRY
        } else {
            &QUIRKY
        }
    } else {
        match st_sample.forest_kind {
            ForestKind::Palm => &PALMS,
            ForestKind::Savannah => &ACACIAS,
            ForestKind::Oak if QUIRKY_RAND.get(st_seed) % 16 == 7 => &OAK_STUMPS,
            ForestKind::Oak if QUIRKY_RAND.get(st_seed) % 8 == 7 => &FRUIT_TREES,
            ForestKind::Oak => &OAKS,
            ForestKind::Pine => &PINES,
            ForestKind::SnowPine => &SNOW_PINES,
            ForestKind::Mangrove => &MANGROVE_TREES,
        }
    };

    Some(StructureInfo {
        pos: st_pos3d,
        seed: st_seed,
        meta: StructureMeta::Volume {
            units: UNIT_CHOOSER.get(st_seed),
            volume: &volumes[(VOLUME_RAND.get(st_seed) / 13) as usize % volumes.len()],
        },
    })
}

fn st_asset(path: &str, offset: impl Into<Vec3<i32>>) -> Arc<Structure> {
    assets::load_map(path, |s: Structure| s.with_center(offset.into()))
        .expect("Failed to load structure asset")
}

lazy_static! {
    pub static ref OAKS: Vec<Arc<Structure>> = vec![
        // green oaks
        assets::load_map("world.tree.oak_green.1", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world.tree.oak_green.2", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world.tree.oak_green.3", |s: Structure| s
            .with_center(Vec3::new(16, 20, 14)))
        .unwrap(),
        assets::load_map("world.tree.oak_green.4", |s: Structure| s
            .with_center(Vec3::new(18, 21, 14)))
        .unwrap(),
        assets::load_map("world.tree.oak_green.5", |s: Structure| s
            .with_center(Vec3::new(18, 18, 14)))
        .unwrap(),
        assets::load_map("world.tree.oak_green.6", |s: Structure| s
            .with_center(Vec3::new(16, 21, 14)))
        .unwrap(),
        assets::load_map("world.tree.oak_green.7", |s: Structure| s
            .with_center(Vec3::new(20, 19, 14)))
        .unwrap(),
        assets::load_map("world.tree.oak_green.8", |s: Structure| s
            .with_center(Vec3::new(22, 20, 14)))
        .unwrap(),
        assets::load_map("world.tree.oak_green.9", |s: Structure| s
            .with_center(Vec3::new(26, 26, 14)))
        .unwrap(),
    ];

    pub static ref OAK_STUMPS: Vec<Arc<Structure>> = vec![
        // oak stumps
        assets::load_map("world.tree.oak_stump.1", |s: Structure| s
            .with_center(Vec3::new(15, 18, 10)))
        .unwrap(),
        assets::load_map("world.tree.oak_stump.2", |s: Structure| s
            .with_center(Vec3::new(15, 18, 10)))
        .unwrap(),
        assets::load_map("world.tree.oak_stump.3", |s: Structure| s
            .with_center(Vec3::new(16, 20, 10)))
        .unwrap(),
        assets::load_map("world.tree.oak_stump.4", |s: Structure| s
            .with_center(Vec3::new(18, 21, 10)))
        .unwrap(),
        assets::load_map("world.tree.oak_stump.5", |s: Structure| s
            .with_center(Vec3::new(18, 18, 10)))
        .unwrap(),
        assets::load_map("world.tree.oak_stump.6", |s: Structure| s
            .with_center(Vec3::new(16, 21, 10)))
        .unwrap(),
        assets::load_map("world.tree.oak_stump.7", |s: Structure| s
            .with_center(Vec3::new(20, 19, 10)))
        .unwrap(),
        assets::load_map("world.tree.oak_stump.8", |s: Structure| s
            .with_center(Vec3::new(22, 20, 10)))
        .unwrap(),
        assets::load_map("world.tree.oak_stump.9", |s: Structure| s
            .with_center(Vec3::new(26, 26, 10)))
        .unwrap(),
    ];

    pub static ref PINES: Vec<Arc<Structure>> = vec![
        // green pines
        assets::load_map("world.tree.pine_green.1", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world.tree.pine_green.2", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world.tree.pine_green.3", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world.tree.pine_green.4", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world.tree.pine_green.5", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world.tree.pine_green.6", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world.tree.pine_green.7", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world.tree.pine_green.8", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        /*
        // green pines 2
         assets::load_map("world/tree/pine_green_2/1", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/2", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/3", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/4", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/5", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/6", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/7", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/8", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // blue pines
        assets::load_map("world/tree/pine_blue/1", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/2", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/3", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/4", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/5", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/6", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/7", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/8", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        */
    ];
      /*
        // temperate small
        assets::load_map("world/tree/temperate_small/1", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/2", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/3", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/4", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/5", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/6", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        // birch
        assets::load_map("world/tree/birch/1", |s: Structure| s
            .with_center(Vec3::new(12, 9, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/2", |s: Structure| s
            .with_center(Vec3::new(11, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/3", |s: Structure| s
            .with_center(Vec3::new(9, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/4", |s: Structure| s
            .with_center(Vec3::new(9, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/5", |s: Structure| s
            .with_center(Vec3::new(9, 11, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/6", |s: Structure| s
            .with_center(Vec3::new(9, 9, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/7", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/8", |s: Structure| s
            .with_center(Vec3::new(9, 9, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/9", |s: Structure| s
            .with_center(Vec3::new(9, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/10", |s: Structure| s
            .with_center(Vec3::new(10, 9, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/11", |s: Structure| s
            .with_center(Vec3::new(9, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/12", |s: Structure| s
            .with_center(Vec3::new(10, 9, 10)))
        .unwrap(),
        // poplar
        assets::load_map("world/tree/poplar/1", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/2", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/3", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/4", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/5", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/6", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/7", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/8", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/9", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/10", |s: Structure| s
            .with_center(Vec3::new(7, 7, 10)))
        .unwrap(),
        */

    pub static ref PALMS: Vec<Arc<Structure>> = vec![
        // palm trees
        assets::load_map("world.tree.desert_palm.1", |s: Structure| s
            .with_center(Vec3::new(12, 12, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.2", |s: Structure| s
            .with_center(Vec3::new(12, 10, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.3", |s: Structure| s
            .with_center(Vec3::new(12, 12, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.4", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.5", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.6", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.7", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.8", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.9", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world.tree.desert_palm.10", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
    ];

    pub static ref SNOW_PINES: Vec<Arc<Structure>> = vec![
        // snow pines
        st_asset("world.tree.snow_pine.1", (15, 15, 14)),
        st_asset("world.tree.snow_pine.2", (15, 15, 14)),
        st_asset("world.tree.snow_pine.3", (17, 15, 12)),
        st_asset("world.tree.snow_pine.4", (10, 8, 12)),
        st_asset("world.tree.snow_pine.5", (12, 12, 12)),
        st_asset("world.tree.snow_pine.6", (11, 10, 12)),
        st_asset("world.tree.snow_pine.7", (16, 15, 12)),
        st_asset("world.tree.snow_pine.8", (12, 10, 12)),
    ];

    pub static ref ACACIAS: Vec<Arc<Structure>> = vec![
        // acias
        st_asset("world.tree.acacia.1", (16, 17, 1)),
        st_asset("world.tree.acacia.2", (5, 6, 1)),
        st_asset("world.tree.acacia.3", (5, 6, 1)),
        st_asset("world.tree.acacia.4", (15, 16, 1)),
        st_asset("world.tree.acacia.5", (19, 18, 1)),
    ];

    pub static ref FRUIT_TREES: Vec<Arc<Structure>> = vec![
        // fruit trees
        st_asset("world.tree.fruit.1", (5, 5, 7)),
        st_asset("world.tree.fruit.2", (6, 6, 7)),
        st_asset("world.tree.fruit.3", (6, 7, 7)),
        st_asset("world.tree.fruit.4", (3, 3, 7)),
        st_asset("world.tree.fruit.5", (6, 8, 7)),
        st_asset("world.tree.fruit.6", (7, 7, 7)),
    ];

        /*
        // snow birches -> need roots!
        assets::load_map("world/tree/snow_birch/1", |s: Structure| s
            .with_center(Vec3::new(12, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/2", |s: Structure| s
            .with_center(Vec3::new(11, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/3", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/4", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/5", |s: Structure| s
            .with_center(Vec3::new(9, 11, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/6", |s: Structure| s
            .with_center(Vec3::new(9, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/7", |s: Structure| s
            .with_center(Vec3::new(10, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/8", |s: Structure| s
            .with_center(Vec3::new(9, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/9", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/10", |s: Structure| s
            .with_center(Vec3::new(10, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/11", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/12", |s: Structure| s
            .with_center(Vec3::new(10, 9, 4)))
        .unwrap(),
        // willows
        assets::load_map("world/tree/willow/1", |s: Structure| s
            .with_center(Vec3::new(15, 14, 1)))
        .unwrap(),
        assets::load_map("world/tree/willow/2", |s: Structure| s
            .with_center(Vec3::new(11, 12, 1)))
        .unwrap(),
    ];
    */

    pub static ref MANGROVE_TREES: Vec<Arc<Structure>> = vec![
        // oak stumps
        assets::load_map("world.tree.mangroves.1", |s: Structure| s
            .with_center(Vec3::new(18, 18, 8)))
        .unwrap(),
        assets::load_map("world.tree.mangroves.2", |s: Structure| s
            .with_center(Vec3::new(16, 17, 7)))
        .unwrap(),
        assets::load_map("world.tree.mangroves.3", |s: Structure| s
            .with_center(Vec3::new(18, 18, 8)))
        .unwrap(),
        assets::load_map("world.tree.mangroves.4", |s: Structure| s
            .with_center(Vec3::new(18, 16, 8)))
        .unwrap(),
        assets::load_map("world.tree.mangroves.5", |s: Structure| s
            .with_center(Vec3::new(19, 20, 9)))
        .unwrap(),
        assets::load_map("world.tree.mangroves.6", |s: Structure| s
            .with_center(Vec3::new(18, 18, 9)))
        .unwrap(),
        assets::load_map("world.tree.mangroves.7", |s: Structure| s
            .with_center(Vec3::new(18, 17, 9)))
        .unwrap(),
        assets::load_map("world.tree.mangroves.8", |s: Structure| s
            .with_center(Vec3::new(18, 18, 9)))
        .unwrap(),
    ];

    pub static ref QUIRKY: Vec<Arc<Structure>> = vec![
        st_asset("world.structure.natural.tower-ruin", (11, 14, 5)),
        st_asset("world.structure.natural.witch-hut", (10, 13, 9)),
    ];

    pub static ref QUIRKY_DRY: Vec<Arc<Structure>> = vec![
        st_asset("world.structure.natural.ribcage-small", (7, 13, 4)),
        st_asset("world.structure.natural.ribcage-large", (13, 19, 8)),
        st_asset("world.structure.natural.skull-large", (15, 20, 4)),
    ];


}
