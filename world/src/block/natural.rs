use super::{BlockGen, StructureInfo, ZCache};
use crate::{
    all::ForestKind,
    column::{ColumnGen, ColumnSample},
    util::{HashCache, RandomPerm, Sampler},
};
use common::{assets, terrain::Structure};
use lazy_static::lazy_static;
use std::sync::Arc;
use vek::*;

static VOLUME_RAND: RandomPerm = RandomPerm::new(0);
static UNIT_RAND: RandomPerm = RandomPerm::new(1);
static QUIRKY_RAND: RandomPerm = RandomPerm::new(2);

pub fn structure_gen<'a>(
    column_gen: &ColumnGen<'a>,
    column_cache: &mut HashCache<Vec2<i32>, Option<ColumnSample<'a>>>,
    idx: usize,
    st_pos: Vec2<i32>,
    st_seed: u32,
    structure_samples: &[Option<ColumnSample>; 9],
) -> Option<StructureInfo> {
    let st_sample = &structure_samples[idx].as_ref()?;

    // Assuming it's a tree... figure out when it SHOULDN'T spawn
    if st_sample.tree_density < 0.5 + (st_seed as f32 / 1000.0).fract() * 0.2
        || st_sample.alt < st_sample.water_level
        || st_sample.spawn_rate < 0.5
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

    let volumes: &'static [_] = match st_sample.forest_kind {
        ForestKind::Palm => &PALMS,
        ForestKind::Savannah => &PALMS,
        ForestKind::Oak if QUIRKY_RAND.get(st_seed) % 64 == 0 => &QUIRKY,
        ForestKind::Oak if QUIRKY_RAND.get(st_seed) % 16 == 0 => &OAK_STUMPS,
        ForestKind::Oak => &OAKS,
        ForestKind::Pine => &PINES,
        ForestKind::SnowPine => &SNOW_PINES,
    };

    const UNIT_CHOICES: [(Vec2<i32>, Vec2<i32>); 1] = [(Vec2 { x: 1, y: 0 }, Vec2 { x: 0, y: 1 })];

    Some(StructureInfo {
        pos: st_pos3d,
        seed: st_seed,
        units: UNIT_CHOICES[UNIT_RAND.get(st_seed) as usize % UNIT_CHOICES.len()],
        volume: &volumes[VOLUME_RAND.get(st_seed) as usize % volumes.len()],
    })
}

lazy_static! {
    pub static ref OAKS: Vec<Arc<Structure>> = vec![
        // green oaks
        assets::load_map("world/tree/oak_green/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/3.vox", |s: Structure| s
            .with_center(Vec3::new(16, 20, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/4.vox", |s: Structure| s
            .with_center(Vec3::new(18, 21, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/5.vox", |s: Structure| s
            .with_center(Vec3::new(18, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/6.vox", |s: Structure| s
            .with_center(Vec3::new(16, 21, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/7.vox", |s: Structure| s
            .with_center(Vec3::new(20, 19, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/8.vox", |s: Structure| s
            .with_center(Vec3::new(22, 20, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/9.vox", |s: Structure| s
            .with_center(Vec3::new(26, 26, 14)))
        .unwrap(),
    ];

    pub static ref OAK_STUMPS: Vec<Arc<Structure>> = vec![
        // oak stumps
        assets::load_map("world/tree/oak_stump/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 10)))
        .unwrap(),
        assets::load_map("world/tree/oak_stump/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 10)))
        .unwrap(),
        assets::load_map("world/tree/oak_stump/3.vox", |s: Structure| s
            .with_center(Vec3::new(16, 20, 10)))
        .unwrap(),
        assets::load_map("world/tree/oak_stump/4.vox", |s: Structure| s
            .with_center(Vec3::new(18, 21, 10)))
        .unwrap(),
        assets::load_map("world/tree/oak_stump/5.vox", |s: Structure| s
            .with_center(Vec3::new(18, 18, 10)))
        .unwrap(),
        assets::load_map("world/tree/oak_stump/6.vox", |s: Structure| s
            .with_center(Vec3::new(16, 21, 10)))
        .unwrap(),
        assets::load_map("world/tree/oak_stump/7.vox", |s: Structure| s
            .with_center(Vec3::new(20, 19, 10)))
        .unwrap(),
        assets::load_map("world/tree/oak_stump/8.vox", |s: Structure| s
            .with_center(Vec3::new(22, 20, 10)))
        .unwrap(),
        assets::load_map("world/tree/oak_stump/9.vox", |s: Structure| s
            .with_center(Vec3::new(26, 26, 10)))
        .unwrap(),
    ];

    pub static ref PINES: Vec<Arc<Structure>> = vec![
        // green pines
        assets::load_map("world/tree/pine_green/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        /*
        // green pines 2
         assets::load_map("world/tree/pine_green_2/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // blue pines
        assets::load_map("world/tree/pine_blue/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        */
    ];
      /*
        // temperate small
        assets::load_map("world/tree/temperate_small/1.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/2.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/3.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/4.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/5.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/6.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        // birch
        assets::load_map("world/tree/birch/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 9, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/3.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/4.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/5.vox", |s: Structure| s
            .with_center(Vec3::new(9, 11, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/6.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/8.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/9.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/11.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/birch/12.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 10)))
        .unwrap(),
        // poplar
        assets::load_map("world/tree/poplar/1.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/2.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/3.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/4.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/5.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/6.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/7.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/8.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/9.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/10.vox", |s: Structure| s
            .with_center(Vec3::new(7, 7, 10)))
        .unwrap(),
        */

    pub static ref PALMS: Vec<Arc<Structure>> = vec![
        // palm trees
        assets::load_map("world/tree/desert_palm/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/2.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/3.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/5.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/6.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/8.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/9.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
    ];

    pub static ref SNOW_PINES: Vec<Arc<Structure>> = vec![
        // snow pines
        assets::load_map("world/tree/snow_pine/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
    ];
        /*
        // snow birches -> need roots!
        assets::load_map("world/tree/snow_birch/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/3.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/4.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/5.vox", |s: Structure| s
            .with_center(Vec3::new(9, 11, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/6.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/8.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/9.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/11.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/12.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 4)))
        .unwrap(),
        // willows
        assets::load_map("world/tree/willow/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 14, 1)))
        .unwrap(),
        assets::load_map("world/tree/willow/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 12, 1)))
        .unwrap(),
    ];
    */

    pub static ref QUIRKY: Vec<Arc<Structure>> = vec![
        assets::load_map("world/structure/natural/tower_ruin.vox", |s: Structure| s
            .with_center(Vec3::new(11, 14, 7)))
        .unwrap(),
    ];
}
