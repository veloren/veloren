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
use world::{
    IndexRef, World,
    civ::airship_travel::{AirshipDockingSide, Airships},
    site::{PlotKind, plot::PlotKindMeta},
    CONFIG, IndexRef, World,
    civ::airship_travel::AirshipSpawningLocation,
    site::{PlotKind, SiteKind},
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
        // Get the spawning locations for airships.
        let spawning_locations = world.civs().airships.airship_spawning_locations();

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

    pub fn airship_spawning_locations(&self, world: &World) -> Vec<AirshipSpawningLocation> {
        world.civs().airships.airship_spawning_locations()
    }

    /// Creates an airship and captain NPC. The NPCs are created at the
    /// approximate correct position, but the final 3D position is set
    /// in register_airship_captain, which is called after all the airship
    /// NPCs are created or loaded from the rtsim data.
    pub fn spawn_airship(
        &mut self,
        spawning_location: &AirshipSpawningLocation,
        rng: &mut impl Rng,
    ) -> (NpcId, NpcId) {
        let npc_wpos3d = spawning_location.pos.with_z(spawning_location.height);

        let vehicle_id = self.npcs.create_npc(Npc::new(
            rng.gen(),
            npc_wpos3d,
            Body::Ship(comp::body::ship::Body::DefaultAirship),
            Role::Vehicle,
        ));

        let species = comp::humanoid::ALL_SPECIES.choose(&mut *rng).unwrap();
        let npc_id = self.npcs.create_npc(
            Npc::new(
                rng.gen(),
                npc_wpos3d,
                Body::Humanoid(comp::humanoid::Body::random_with(rng, species)),
                Role::Civilised(Some(Profession::Captain)),
            )
            // .with_home(spawning_location.site_id)
            .with_personality(Personality::random_good(rng)),
        );

        // The captain is mounted on the airship
        self.npcs
            .mounts
            .steer(vehicle_id, npc_id)
            .expect("We just created these npcs!");

        (npc_id, vehicle_id)
    }
}
