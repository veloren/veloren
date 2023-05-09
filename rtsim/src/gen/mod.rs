pub mod faction;
pub mod name;
pub mod site;

use crate::data::{
    faction::Faction,
    npc::{Npc, Npcs, Profession, Vehicle},
    site::Site,
    Data, Nature, CURRENT_VERSION,
};
use common::{
    comp::{self, Body},
    grid::Grid,
    resources::TimeOfDay,
    rtsim::{Personality, Role, WorldSettings},
    terrain::{BiomeKind, CoordinateConversions, TerrainChunkSize},
    vol::RectVolSize,
};
use rand::prelude::*;
use tracing::info;
use vek::*;
use world::{site::SiteKind, site2::PlotKind, IndexRef, World, CONFIG};

impl Data {
    pub fn generate(settings: &WorldSettings, world: &World, index: IndexRef) -> Self {
        let mut seed = [0; 32];
        seed.iter_mut()
            .zip(&mut index.seed.to_le_bytes())
            .for_each(|(dst, src)| *dst = *src);
        let mut rng = SmallRng::from_seed(seed);

        let mut this = Self {
            version: CURRENT_VERSION,
            nature: Nature::generate(world),
            npcs: Npcs {
                npcs: Default::default(),
                vehicles: Default::default(),
                npc_grid: Grid::new(Vec2::zero(), Default::default()),
                character_map: Default::default(),
            },
            sites: Default::default(),
            factions: Default::default(),
            reports: Default::default(),

            tick: 0,
            time_of_day: TimeOfDay(settings.start_time),
            should_purge: false,
        };

        let initial_factions = (0..16)
            .map(|_| {
                let faction = Faction::generate(world, index, &mut rng);
                let wpos = world
                    .sim()
                    .get_size()
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                        rng.gen_range(0..(e * sz) as i32)
                    });
                (wpos, this.factions.create(faction))
            })
            .collect::<Vec<_>>();
        info!("Generated {} rtsim factions.", this.factions.len());

        // Register sites with rtsim
        for (world_site_id, _) in index.sites.iter() {
            let site = Site::generate(
                world_site_id,
                world,
                index,
                &initial_factions,
                &this.factions,
                &mut rng,
            );
            this.sites.create(site);
        }
        info!(
            "Registering {} rtsim sites from world sites.",
            this.sites.len()
        );
        // Spawn some test entities at the sites
        for (site_id, site, site2) in this.sites.iter()
        // TODO: Stupid. Only find site2 towns
        .filter_map(|(site_id, site)| Some((site_id, site, site.world_site
            .and_then(|ws| match &index.sites.get(ws).kind {
                SiteKind::Refactor(site2)
                | SiteKind::CliffTown(site2)
                | SiteKind::SavannahPit(site2)
                | SiteKind::DesertCity(site2) => Some(site2),
                _ => None,
            })?)))
        {
            let Some(good_or_evil) = site
                .faction
                .and_then(|f| this.factions.get(f))
                .map(|f| f.good_or_evil)
            else { continue };

            let rand_wpos = |rng: &mut SmallRng, matches_plot: fn(&PlotKind) -> bool| {
                let wpos2d = site2
                    .plots()
                    .filter(|plot| matches_plot(plot.kind()))
                    .choose(&mut thread_rng())
                    .map(|plot| site2.tile_center_wpos(plot.root_tile()))
                    .unwrap_or_else(|| site.wpos.map(|e| e + rng.gen_range(-10..10)));
                wpos2d
                    .map(|e| e as f32 + 0.5)
                    .with_z(world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
            };
            let random_humanoid = |rng: &mut SmallRng| {
                let species = comp::humanoid::ALL_SPECIES.choose(&mut *rng).unwrap();
                Body::Humanoid(comp::humanoid::Body::random_with(rng, species))
            };
            let matches_buildings = (|kind: &PlotKind| {
                matches!(
                    kind,
                    PlotKind::House(_) | PlotKind::Workshop(_) | PlotKind::Plaza
                )
            }) as _;
            let matches_plazas = (|kind: &PlotKind| matches!(kind, PlotKind::Plaza)) as _;
            if good_or_evil {
                for _ in 0..site2.plots().len() {
                    this.npcs.create_npc(
                        Npc::new(
                            rng.gen(),
                            rand_wpos(&mut rng, matches_buildings),
                            random_humanoid(&mut rng),
                            Role::Civilised(Some(match rng.gen_range(0..20) {
                                0 => Profession::Hunter,
                                1 => Profession::Blacksmith,
                                2 => Profession::Chef,
                                3 => Profession::Alchemist,
                                5..=8 => Profession::Farmer,
                                9..=10 => Profession::Herbalist,
                                11..=16 => Profession::Guard,
                                _ => Profession::Adventurer(rng.gen_range(0..=3)),
                            })),
                        )
                        .with_faction(site.faction)
                        .with_home(site_id)
                        .with_personality(Personality::random(&mut rng)),
                    );
                }
            } else {
                for _ in 0..15 {
                    this.npcs.create_npc(
                        Npc::new(
                            rng.gen(),
                            rand_wpos(&mut rng, matches_buildings),
                            random_humanoid(&mut rng),
                            Role::Civilised(Some(Profession::Cultist)),
                        )
                        .with_personality(Personality::random_evil(&mut rng))
                        .with_faction(site.faction)
                        .with_home(site_id),
                    );
                }
            }
            // Merchants
            if good_or_evil {
                for _ in 0..(site2.plots().len() / 6) + 1 {
                    this.npcs.create_npc(
                        Npc::new(
                            rng.gen(),
                            rand_wpos(&mut rng, matches_plazas),
                            random_humanoid(&mut rng),
                            Role::Civilised(Some(Profession::Merchant)),
                        )
                        .with_home(site_id)
                        .with_personality(Personality::random_good(&mut rng)),
                    );
                }
            }

            if rng.gen_bool(0.4) {
                let wpos = rand_wpos(&mut rng, matches_plazas) + Vec3::unit_z() * 50.0;
                let vehicle_id = this
                    .npcs
                    .create_vehicle(Vehicle::new(wpos, comp::body::ship::Body::DefaultAirship));

                this.npcs.create_npc(
                    Npc::new(
                        rng.gen(),
                        wpos,
                        random_humanoid(&mut rng),
                        Role::Civilised(Some(Profession::Captain)),
                    )
                    .with_home(site_id)
                    .with_personality(Personality::random_good(&mut rng))
                    .steering(vehicle_id),
                );
            }
        }

        for (site_id, site) in this.sites.iter()
        // TODO: Stupid
        .filter(|(_, site)| site.world_site.map_or(false, |ws|
        matches!(&index.sites.get(ws).kind, SiteKind::Dungeon(_))))
        {
            let rand_wpos = |rng: &mut SmallRng| {
                let wpos2d = site.wpos.map(|e| e + rng.gen_range(-10..10));
                wpos2d
                    .map(|e| e as f32 + 0.5)
                    .with_z(world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
            };

            let species = [
                comp::body::bird_large::Species::Phoenix,
                comp::body::bird_large::Species::Cockatrice,
                comp::body::bird_large::Species::Roc,
            ]
            .choose(&mut rng)
            .unwrap();
            this.npcs.create_npc(
                Npc::new(
                    rng.gen(),
                    rand_wpos(&mut rng),
                    Body::BirdLarge(comp::body::bird_large::Body::random_with(&mut rng, species)),
                    Role::Wild,
                )
                .with_home(site_id),
            );
        }

        // Spawn monsters into the world
        for _ in 0..100 {
            // Try a few times to find a location that's not underwater
            if let Some((wpos, chunk)) = (0..10)
                .map(|_| world.sim().get_size().map(|sz| rng.gen_range(0..sz as i32)))
                .find_map(|pos| Some((pos, world.sim().get(pos).filter(|c| !c.is_underwater())?)))
                .map(|(pos, chunk)| {
                    let wpos2d = pos.cpos_to_wpos_center();
                    (
                        wpos2d
                            .map(|e| e as f32 + 0.5)
                            .with_z(world.sim().get_alt_approx(wpos2d).unwrap_or(0.0)),
                        chunk,
                    )
                })
            {
                let biome = chunk.get_biome();
                let Some(species) = [
                    Some(comp::body::biped_large::Species::Ogre),
                    Some(comp::body::biped_large::Species::Cyclops),
                    Some(comp::body::biped_large::Species::Wendigo).filter(|_| biome == BiomeKind::Taiga),
                    Some(comp::body::biped_large::Species::Cavetroll),
                    Some(comp::body::biped_large::Species::Mountaintroll).filter(|_| biome == BiomeKind::Mountain),
                    Some(comp::body::biped_large::Species::Swamptroll).filter(|_| biome == BiomeKind::Swamp),
                    Some(comp::body::biped_large::Species::Blueoni),
                    Some(comp::body::biped_large::Species::Redoni),
                    Some(comp::body::biped_large::Species::Tursus).filter(|_| chunk.temp < CONFIG.snow_temp),
                ]
                    .into_iter()
                    .flatten()
                    .choose(&mut rng)
                else { continue };

                this.npcs.create_npc(Npc::new(
                    rng.gen(),
                    wpos,
                    Body::BipedLarge(comp::body::biped_large::Body::random_with(
                        &mut rng, &species,
                    )),
                    Role::Monster,
                ));
            }
        }

        info!("Generated {} rtsim NPCs.", this.npcs.len());

        this
    }
}
