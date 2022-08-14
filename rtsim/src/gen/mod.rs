pub mod site;

use crate::data::{
    npc::{Npcs, Npc, Profession},
    site::{Sites, Site},
    Data,
    Nature,
};
use hashbrown::HashMap;
use rand::prelude::*;
use tracing::info;
use world::{
    site::SiteKind,
    IndexRef,
    World,
};

impl Data {
    pub fn generate(world: &World, index: IndexRef) -> Self {
        let mut seed = [0; 32];
        seed.iter_mut().zip(&mut index.seed.to_le_bytes()).for_each(|(dst, src)| *dst = *src);
        let mut rng = SmallRng::from_seed(seed);

        let mut this = Self {
            nature: Nature::generate(world),
            npcs: Npcs { npcs: Default::default() },
            sites: Sites { sites: Default::default() },

            time_of_day: Default::default(),
        };

        // Register sites with rtsim
        for (world_site_id, _) in index
            .sites
            .iter()
        {
            let site = Site::generate(world_site_id, world, index);
            this.sites.create(site);
        }
        info!("Registering {} rtsim sites from world sites.", this.sites.len());

        // Spawn some test entities at the sites
        for (site_id, site) in this.sites.iter() {
            let rand_wpos = |rng: &mut SmallRng| {
                let wpos2d = site.wpos.map(|e| e + rng.gen_range(-10..10));
                wpos2d.map(|e| e as f32 + 0.5)
                    .with_z(world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
            };
            for _ in 0..10 {

                this.npcs.create(Npc::new(rng.gen(), rand_wpos(&mut rng)).with_home(site_id).with_profession(match rng.gen_range(0..10) {
                    0 => Profession::Hunter,
                    1..=4 => Profession::Farmer,
                    _ => Profession::Guard,
                }));
            }
            this.npcs.create(Npc::new(rng.gen(), rand_wpos(&mut rng)).with_home(site_id).with_profession(Profession::Merchant));
        }

        this
    }
}
