use super::{BlockGen, StructureInfo, StructureMeta};
use crate::{
    all::ForestKind,
    column::{ColumnGen, ColumnSample},
    util::{RandomPerm, Sampler, SmallCache, UnitChooser},
    CONFIG,
};
use common::{assets, assets::Asset, terrain::Structure};
use lazy_static::lazy_static;
use ron;
use serde::Deserialize;
use std::{fs::File, io::BufReader, sync::Arc, u32};
use vek::*;

static VOLUME_RAND: RandomPerm = RandomPerm::new(0xDB21C052);
static UNIT_CHOOSER: UnitChooser = UnitChooser::new(0x700F4EC7);
static QUIRKY_RAND: RandomPerm = RandomPerm::new(0xA634460F);

pub fn structure_gen<'a>(
    column_gen: &ColumnGen<'a>,
    column_cache: &mut SmallCache<Option<ColumnSample<'a>>>,
    st_pos: Vec2<i32>,
    st_seed: u32,
    st_sample: &ColumnSample,
) -> Option<StructureInfo> {
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
        0.0,
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
            ForestKind::Oak if QUIRKY_RAND.get(st_seed) % 19 == 7 => &FRUIT_TREES,
            ForestKind::Oak if QUIRKY_RAND.get(st_seed) % 14 == 7 => &BIRCHES,
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

#[derive(Deserialize)]
struct StructureSpec {
    specifier: String,
    center: [i32; 3],
}
#[derive(Deserialize)]
struct StructuresSpec(Vec<StructureSpec>);

impl Asset for StructuresSpec {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        ron::de::from_reader(buf_reader).map_err(assets::Error::parse_error)
    }
}

fn load_structures(specifier: &str) -> Vec<Arc<Structure>> {
    let spec = assets::load::<StructuresSpec>(&["world.manifests.", specifier].concat());
    return spec
        .unwrap()
        .0
        .iter()
        .map(|sp| {
            assets::load_map(&sp.specifier[..], |s: Structure| {
                s.with_center(Vec3::from(sp.center))
            })
            .unwrap()
        })
        .collect();
}

lazy_static! {
    pub static ref OAKS: Vec<Arc<Structure>> = load_structures("oaks");
    pub static ref OAK_STUMPS: Vec<Arc<Structure>> = load_structures("oak_stumps");
    pub static ref PINES: Vec<Arc<Structure>> = load_structures("pines");
    pub static ref PALMS: Vec<Arc<Structure>> = load_structures("palms");
    pub static ref SNOW_PINES: Vec<Arc<Structure>> = load_structures("snow_pines");
    pub static ref ACACIAS: Vec<Arc<Structure>> = load_structures("acacias");
    pub static ref FRUIT_TREES: Vec<Arc<Structure>> = load_structures("fruit_trees");
    pub static ref BIRCHES: Vec<Arc<Structure>> = load_structures("birch");
    pub static ref MANGROVE_TREES: Vec<Arc<Structure>> = load_structures("mangrove_trees");
    pub static ref QUIRKY: Vec<Arc<Structure>> = load_structures("quirky");
    pub static ref QUIRKY_DRY: Vec<Arc<Structure>> = load_structures("quirky_dry");
}
