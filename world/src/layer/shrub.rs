use crate::{
    util::{seed_expan, RandomPerm, Sampler, StructureGen2d, UnitChooser},
    Canvas,
};
use common::{
    assets::AssetHandle,
    terrain::structure::{Structure, StructuresGroup},
    vol::ReadVol,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use vek::*;

lazy_static! {
    static ref SHRUBS: AssetHandle<StructuresGroup> = Structure::load_group("shrubs");
}

struct Shrub {
    wpos: Vec3<i32>,
    seed: u32,
}

pub fn apply_shrubs_to(canvas: &mut Canvas, rng: &mut impl Rng) {
    let mut shrub_cache = HashMap::new();

    let shrub_gen = StructureGen2d::new(canvas.index().seed, 8, 4);

    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        for (wpos, seed) in std::array::IntoIter::new(shrub_gen.get(wpos2d)) {
            shrub_cache.entry(wpos).or_insert_with(|| {
                let col = info.col_or_gen(wpos)?;

                if RandomPerm::new(seed).chance(37, col.tree_density * 0.3)
                    && col.water_dist.map_or(true, |d| d > 8.0)
                    && col.alt > col.water_level
                    && col.spawn_rate > 0.9
                    && col.path.map_or(true, |(d, _, _, _)| d > 6.0)
                {
                    Some(Shrub {
                        wpos: wpos.with_z(col.alt as i32),
                        seed,
                    })
                } else {
                    None
                }
            });
        }
    });

    for shrub in shrub_cache.values().filter_map(|s| s.as_ref()) {
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(shrub.seed));

        let units = UnitChooser::new(shrub.seed).get(shrub.seed).into();

        let shrubs = SHRUBS.read();
        let structure = shrubs.choose(&mut rng).unwrap();
        canvas.blit_structure(shrub.wpos, structure, shrub.seed, units, true);
    }
}
