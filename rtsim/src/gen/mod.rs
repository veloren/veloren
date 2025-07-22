pub mod faction;
pub mod name;
pub mod site;

use crate::data::{
    CURRENT_VERSION, Data, Nature,
    airship::AirshipSpawningLocation,
    architect::{Population, TrackedPopulation},
    faction::Faction,
    npc::{Npc, Npcs, Profession},
    site::Site,
};
use common::{
    comp::{self, Body},
    resources::TimeOfDay,
    rtsim::{NpcId, Personality, Role, WorldSettings},
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use tracing::info;
use vek::*;
use world::{
    IndexRef, World,
    civ::airship_travel::{AirshipDockingSide, Airships},
    site::{PlotKind, plot::PlotKindMeta},
    util::seed_expan,
};

pub fn wanted_population(world: &World, index: IndexRef) -> Population {
    let mut pop = Population::default();

    let sites = &index.sites;

    // Spawn some npcs at settlements
    for (_, site) in sites.iter()
        // TODO: Stupid. Only find site towns
        .filter(|(_, site)| site.meta().is_some_and(|m| matches!(m, common::terrain::SiteKindMeta::Settlement(_))))
    {
        let town_pop = site.plots().len() as u32;
        let guards = town_pop / 4;
        let adventurers = town_pop / 5;
        let others = town_pop.saturating_sub(guards + adventurers);

        pop.add(TrackedPopulation::Guards, guards);
        pop.add(TrackedPopulation::Adventurers, adventurers);
        pop.add(TrackedPopulation::OtherTownNpcs, others);

        pop.add(TrackedPopulation::Merchants, (town_pop / 6) + 1);
    }

    let pirate_hideouts = sites
        .iter()
        .flat_map(|(_, site)| {
            site.plots()
                .filter(|plot| matches!(plot.kind(), PlotKind::PirateHideout(_)))
        })
        .count() as u32;

    // Pirates
    pop.add(TrackedPopulation::PirateCaptains, pirate_hideouts);
    pop.add(TrackedPopulation::Pirates, 10 * pirate_hideouts);

    // Birds
    let world_area = world.sim().map_size_lg().chunks_len();
    let bird_pop = world_area.div_ceil(2usize.pow(16)) as u32;
    let bird_kind_pop = bird_pop.div_ceil(8);
    pop.add(TrackedPopulation::CloudWyvern, bird_kind_pop);
    pop.add(TrackedPopulation::FrostWyvern, bird_kind_pop);
    pop.add(TrackedPopulation::SeaWyvern, bird_kind_pop);
    pop.add(TrackedPopulation::FlameWyvern, bird_kind_pop);
    pop.add(TrackedPopulation::WealdWyvern, bird_kind_pop);
    pop.add(TrackedPopulation::Phoenix, bird_kind_pop);
    pop.add(TrackedPopulation::Roc, bird_kind_pop);
    pop.add(TrackedPopulation::Cockatrice, bird_kind_pop);

    // Monsters
    pop.add(TrackedPopulation::GigasFrost, 1);
    pop.add(TrackedPopulation::GigasFire, 1);
    pop.add(
        TrackedPopulation::OtherMonsters,
        (world.sim().map_size_lg().chunks_len() / 2usize.pow(13)).clamp(5, 1000) as u32,
    );

    pop
}

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
            npcs: Npcs::default(),
            sites: Default::default(),
            factions: Default::default(),
            reports: Default::default(),
            airship_sim: Default::default(),
            architect: Default::default(),

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

        // Airships
        // Get the spawning locations for the sites with airship docks. It's possible
        // that not all docking positions will be used at all sites based on
        // pairing with routes and how the routes are generated.
        let spawning_locations = this.airship_spawning_locations(world, index);

        // When generating rtsim data from scratch, put an airship (and captain) at each
        // available spawning location. Note this is just to get the initial
        // airship NPCs created. Since the airship route data is not persisted,
        // but the NPCs themselves are, the rtsim data contains airships and captains,
        // but not the routes, and the information about routes and route
        // assignments is generated each time the server is started. This process
        // of resolving the rtsim data to the world data is done in the `migrate`
        // module.

        let mut airship_rng = ChaChaRng::from_seed(seed_expan::rng_state(index.index.seed));
        for spawning_location in spawning_locations.iter() {
            this.spawn_airship(spawning_location, &mut airship_rng);
        }

        this.architect.wanted_population = wanted_population(world, index);

        info!(
            "Generated {} rtsim NPCs to be spawned.",
            this.architect.wanted_population.total()
        );

        this
    }

    /// Get all the places that an airship should be spawned. The site must be a
    /// town or city that could have one or more airship docks. The plot
    /// type must be an airship dock, and the docking position must be one
    /// that the airship can spawn at according to the world airship routes.
    pub fn airship_spawning_locations(
        &self,
        world: &World,
        index: IndexRef,
    ) -> Vec<AirshipSpawningLocation> {
        self.sites
            .iter()
            .filter_map(|(site_id, site)| {
                Some((
                    site_id,
                    site,
                    site.world_site.map(|ws| index.sites.get(ws))?,
                ))
            })
            .flat_map(|(site_id, _, site)| {
                site.plots
                    .values()
                    .filter_map(move |plot| {
                        if let Some(PlotKindMeta::AirshipDock {
                            center,
                            docking_positions,
                            ..
                        }) = plot.kind().meta()
                        {
                            Some(
                                docking_positions
                                    .iter()
                                    .filter_map(move |docking_pos| {
                                        if world
                                            .civs()
                                            .airships
                                            .should_spawn_airship_at_docking_position(
                                                docking_pos,
                                                site.name(),
                                            )
                                        {
                                            let (airship_pos, airship_dir) =
                                                Airships::airship_vec_for_docking_pos(
                                                    docking_pos.map(|i| i as f32),
                                                    center.map(|i| i as f32),
                                                    // This is a temporary choice just to make the
                                                    // spawning location data deterministic.
                                                    // The actual docking side is selected when the
                                                    // route and approach are selected in the
                                                    // migrate module.
                                                    Some(AirshipDockingSide::Starboard),
                                                );
                                            Some(AirshipSpawningLocation {
                                                pos: airship_pos,
                                                dir: airship_dir,
                                                center,
                                                docking_pos: *docking_pos,
                                                site_id,
                                                site_name: site.name().to_string(),
                                            })
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        } else {
                            None
                        }
                    })
                    .flatten()
            })
            .collect::<Vec<_>>()
    }

    /// Creates an airship and captain NPC at the given spawning location. The
    /// location is tempory since the airship will be moved into position
    /// after the npcs are spawned.
    pub fn spawn_airship(
        &mut self,
        spawning_location: &AirshipSpawningLocation,
        rng: &mut impl Rng,
    ) -> (NpcId, NpcId) {
        let vehicle_id = self.npcs.create_npc(Npc::new(
            rng.gen(),
            spawning_location.pos,
            Body::Ship(comp::body::ship::Body::DefaultAirship),
            Role::Vehicle,
        ));
        let airship = self.npcs.get_mut(vehicle_id).unwrap();
        let airship_mount_offset = airship.body.mount_offset();

        let captain_pos = spawning_location.pos
            + Vec3::new(
                spawning_location.dir.x * airship_mount_offset.x,
                spawning_location.dir.y * airship_mount_offset.y,
                airship_mount_offset.z,
            );
        let species = comp::humanoid::ALL_SPECIES.choose(&mut *rng).unwrap();
        let npc_id = self.npcs.create_npc(
            Npc::new(
                rng.gen(),
                captain_pos,
                Body::Humanoid(comp::humanoid::Body::random_with(rng, species)),
                Role::Civilised(Some(Profession::Captain)),
            )
            // .with_home(spawning_location.site_id)
            .with_personality(Personality::random_good(rng)),
        );
        // airship_captains.push((spawning_location.pos, npc_id, vehicle_id));
        self.npcs.get_mut(npc_id).unwrap().dir = spawning_location.dir.xy().normalized();

        // The captain is mounted on the airship
        self.npcs
            .mounts
            .steer(vehicle_id, npc_id)
            .expect("We just created these npcs!");

        self.npcs.get_mut(vehicle_id).unwrap().dir = spawning_location.dir.xy().normalized();

        (npc_id, vehicle_id)
    }
}
