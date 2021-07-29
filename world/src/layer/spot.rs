use crate::{
    sim::{SimChunk, WorldSim},
    util::seed_expan,
    Canvas,
};
use common::{generation::EntityInfo, terrain::Structure};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::ops::Range;
use vek::*;

/// Spots are localised structures that spawn in the world. Conceptually, they
/// fit somewhere between the tree generator and the site generator: an attempt
/// to marry the simplicity of the former with the capability of the latter.
/// They are not globally visible to the game: this means that they do not
/// appear on the map, and cannot interact with rtsim (much).
///
/// To add a new spot, one must:
///
/// 1. Add a new variant to the [`Spot`] enum.
///
/// 2. Add a new entry to [`Spot::generate`] that tells the system where to
/// generate your new spot.
///
/// 3. Add a new arm to the `match` expression in [`Spot::apply_spots_to`] that
/// tells the generator how to generate a spot, including the base structure
/// that composes the spot and the entities that should be spawned there.
#[derive(Copy, Clone, Debug)]
pub enum Spot {
    MerchantCamp,
    SaurokCamp,
}

impl Spot {
    pub fn generate(world: &mut WorldSim) {
        Self::generate_spots(
            Spot::MerchantCamp,
            world,
            10.0,
            |g, c| g < 0.25 && !c.near_cliffs() && !c.river.near_water() && !c.path.0.is_way(),
            false,
        );
        Self::generate_spots(
            Spot::SaurokCamp,
            world,
            10.0,
            |g, c| g < 0.25 && !c.near_cliffs() && !c.river.near_water() && !c.path.0.is_way(),
            false,
        );
    }

    fn generate_spots(
        // What kind of spot are we generating?
        spot: Spot,
        world: &mut WorldSim,
        // How often should this spot appear (per square km, on average)?
        freq: f32,
        // What tests should we perform to see whether we can spawn the spot here? The two
        // parameters are the gradient of the terrain and the [`SimChunk`] of the candidate
        // location.
        mut valid: impl FnMut(f32, &SimChunk) -> bool,
        // Should we allow trees to spawn close to the spot?
        trees: bool,
    ) {
        let world_size = world.get_size();
        for _ in 0..(world_size.product() as f32 / 32.0f32.powi(2) * freq).ceil() as u64 {
            let pos = world_size.map(|e| world.rng.gen_range(0..e as i32));
            if let Some((_, chunk)) = world
                .get_gradient_approx(pos)
                .zip(world.get_mut(pos))
                .filter(|(grad, chunk)| valid(*grad, chunk))
            {
                chunk.spot = Some(spot);
                if !trees {
                    chunk.tree_density = 0.0;
                }
            }
        }
    }
}

pub fn apply_spots_to(canvas: &mut Canvas, _dynamic_rng: &mut impl Rng) {
    let nearby_spots = canvas.nearby_spots().collect::<Vec<_>>();

    for (spot_wpos2d, spot, seed) in nearby_spots.iter().copied() {
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));

        #[derive(Default)]
        struct SpotConfig<'a> {
            // The manifest containing a list of possible base structures for the spot (one will be
            // chosen)
            base_structures: Option<&'a str>,
            // The maximum distance from the centre of the spot that entities will spawn
            entity_radius: f32,
            // The entities that should be spawned in the spot, from closest to furthest
            // (count_range, spec)
            entities: &'a [(Range<u32>, &'a str)],
        }

        let spot_config = match spot {
            Spot::MerchantCamp => SpotConfig {
                base_structures: Some("trees.quirky"),
                entity_radius: 6.0,
                entities: &[
                    (1..3, "common.entity.village.merchant"),
                    (2..5, "common.entity.village.villager"),
                ],
            },
            Spot::SaurokCamp => SpotConfig {
                base_structures: Some("dungeon_entrances.grassland"),
                entity_radius: 12.0,
                entities: &[(4..6, "common.entity.spot.bandit_camp.saurok")],
            },
        };

        // Blit base structure
        if let Some(base_structures) = spot_config.base_structures {
            let structures = Structure::load_group(base_structures).read();
            let structure = structures.choose(&mut rng).unwrap();
            let origin = spot_wpos2d.with_z(
                canvas
                    .col_or_gen(spot_wpos2d)
                    .map(|c| c.alt as i32)
                    .unwrap_or(0),
            );
            canvas.blit_structure(origin, &structure, seed);
        }

        // Spawn entities
        const PHI: f32 = 1.618;
        let dir_offset = rng.gen::<f32>();
        let mut i = 0;
        for (spawn_count, spec) in spot_config.entities {
            let spawn_count = rng.gen_range(spawn_count.clone());

            for _ in 0..spawn_count {
                let dir = Vec2::new(
                    ((dir_offset + i as f32 * PHI) * std::f32::consts::TAU).sin(),
                    ((dir_offset + i as f32 * PHI) * std::f32::consts::TAU).cos(),
                );
                let dist = i as f32 / spawn_count as f32 * spot_config.entity_radius;
                let wpos2d = spot_wpos2d + (dir * dist).map(|e| e.round() as i32);

                let alt = canvas.col_or_gen(wpos2d).map(|c| c.alt as i32).unwrap_or(0);

                if let Some(wpos) = canvas
                    .area()
                    .contains_point(wpos2d)
                    .then(|| canvas.find_spawn_pos(wpos2d.with_z(alt)))
                    .flatten()
                {
                    canvas.spawn(
                        EntityInfo::at(wpos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0))
                            .with_asset_expect(spec),
                    );
                }

                i += 1;
            }
        }
    }
}
