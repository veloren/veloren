pub mod faction;
pub mod site;

use crate::data::{
    faction::{Faction, Factions},
    npc::{Npc, Npcs, Profession},
    site::{Site, Sites},
    Data, Nature,
};
use common::{
    resources::TimeOfDay, rtsim::WorldSettings, terrain::TerrainChunkSize, vol::RectVolSize,
};
use hashbrown::HashMap;
use rand::prelude::*;
use tracing::info;
use vek::*;
use world::{site::SiteKind, IndexRef, World};

impl Data {
    pub fn generate(settings: &WorldSettings, world: &World, index: IndexRef) -> Self {
        let mut seed = [0; 32];
        seed.iter_mut()
            .zip(&mut index.seed.to_le_bytes())
            .for_each(|(dst, src)| *dst = *src);
        let mut rng = SmallRng::from_seed(seed);

        let mut this = Self {
            nature: Nature::generate(world),
            npcs: Npcs {
                npcs: Default::default(),
            },
            sites: Sites {
                sites: Default::default(),
                world_site_map: Default::default(),
            },
            factions: Factions {
                factions: Default::default(),
            },

            time_of_day: TimeOfDay(settings.start_time),
        };

        let initial_factions = (0..10)
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
            let site = Site::generate(world_site_id, world, index, &initial_factions);
            this.sites.create(site);
        }
        info!(
            "Registering {} rtsim sites from world sites.",
            this.sites.len()
        );

        // Spawn some test entities at the sites
        for (site_id, site) in this.sites.iter().take(1) {
            let rand_wpos = |rng: &mut SmallRng| {
                let wpos2d = site.wpos.map(|e| e + rng.gen_range(-10..10));
                wpos2d
                    .map(|e| e as f32 + 0.5)
                    .with_z(world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
            };
            for _ in 0..1 {
                this.npcs.create(
                    Npc::new(rng.gen(), rand_wpos(&mut rng))
                        .with_faction(site.faction)
                        .with_home(site_id)
                        .with_profession(match 1/*rng.gen_range(0..20)*/ {
                            0 => Profession::Hunter,
                            1 => Profession::Blacksmith,
                            2 => Profession::Chef,
                            3 => Profession::Alchemist,
                            5..=10 => Profession::Farmer,
                            11..=15 => Profession::Guard,
                            _ => Profession::Adventurer(rng.gen_range(0..=3)),
                        }),
                );
            }
            this.npcs.create(
                Npc::new(rng.gen(), rand_wpos(&mut rng))
                    .with_home(site_id)
                    .with_profession(Profession::Merchant),
            );
        }
        info!("Generated {} rtsim NPCs.", this.npcs.len());

        this
    }
}
