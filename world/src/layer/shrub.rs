use crate::{
    all::ForestKind,
    layer::cave::tunnel_bounds_at,
    util::{gen_cache::StructureGenCache, seed_expan, Sampler, StructureGen2d, UnitChooser},
    Canvas,
};
use common::{
    assets::AssetHandle,
    terrain::structure::{Structure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use vek::*;

lazy_static! {
    static ref JUNGLE_SHRUBS: AssetHandle<StructuresGroup> = Structure::load_group("shrubs.jungle");
    static ref SAVANNAH_SHRUBS: AssetHandle<StructuresGroup> =
        Structure::load_group("shrubs.savannah");
    static ref TEMPERATE_SHRUBS: AssetHandle<StructuresGroup> =
        Structure::load_group("shrubs.temperate");
    static ref TAIGA_SHRUBS: AssetHandle<StructuresGroup> = Structure::load_group("shrubs.taiga");
}

struct Shrub {
    wpos: Vec3<i32>,
    seed: u32,
    kind: ForestKind,
}

pub fn apply_shrubs_to(canvas: &mut Canvas, _dynamic_rng: &mut impl Rng) {
    let mut shrub_gen = StructureGenCache::new(StructureGen2d::new(canvas.index().seed, 8, 4));

    let info = canvas.info();

    canvas.foreach_col(|_, wpos2d, _| {
        shrub_gen.get(wpos2d, |wpos, seed| {
            let col = info.col_or_gen(wpos)?;

            let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));

            const BASE_SHRUB_DENSITY: f64 = 0.15;
            if rng.gen_bool((BASE_SHRUB_DENSITY * col.tree_density as f64).clamped(0.0, 1.0))
                && col.water_dist.map_or(true, |d| d > 8.0)
                && col.alt > col.water_level
                && col.spawn_rate > 0.9
                && col.path.map_or(true, |(d, _, _, _)| d > 6.0)
                && !tunnel_bounds_at(wpos, &info, &info.land())
                    .any(|(_, z_range, _, _)| z_range.contains(&(col.alt as i32 - 1)))
            {
                let kind = *info
                    .chunks()
                    .make_forest_lottery(wpos)
                    .choose_seeded(seed)
                    .as_ref()?;
                if rng.gen_bool(kind.shrub_density_factor() as f64) {
                    Some(Shrub {
                        wpos: wpos.with_z(col.alt as i32),
                        seed,
                        kind,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        });
    });

    for shrub in shrub_gen.generated() {
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(shrub.seed));

        let units = UnitChooser::new(shrub.seed).get(shrub.seed).into();

        let shrubs = match shrub.kind {
            ForestKind::Mangrove => &*JUNGLE_SHRUBS,
            ForestKind::Acacia | ForestKind::Baobab => &*SAVANNAH_SHRUBS,
            ForestKind::Oak | ForestKind::Chestnut => &*TEMPERATE_SHRUBS,
            ForestKind::Pine => &*TAIGA_SHRUBS,
            _ => continue, // TODO: Add more shrub varieties
        }
        .read();

        let structure = shrubs.choose(&mut rng).unwrap();
        canvas.blit_structure(shrub.wpos, structure, shrub.seed, units, true);
    }
}
