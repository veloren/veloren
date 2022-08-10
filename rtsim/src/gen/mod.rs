use crate::data::{Npcs, Npc, Data, Nature};
use hashbrown::HashMap;
use rand::prelude::*;
use world::{
    site::SiteKind,
    IndexRef,
    World,
};

impl Data {
    pub fn generate(index: IndexRef, world: &World) -> Self {
        let mut seed = [0; 32];
        seed.iter_mut().zip(&mut index.seed.to_le_bytes()).for_each(|(dst, src)| *dst = *src);
        let mut rng = SmallRng::from_seed(seed);

        let mut this = Self {
            nature: Nature::generate(world),
            npcs: Npcs { npcs: Default::default() },
        };

        for (site_id, site) in world
            .civs()
            .sites
            .iter()
            .filter_map(|(site_id, site)| site.site_tmp.map(|id| (site_id, &index.sites[id])))
        {
            match &site.kind {
                SiteKind::Refactor(site2) => {
                    let wpos = site.get_origin()
                        .map(|e| e as f32 + 0.5)
                        .with_z(world.sim().get_alt_approx(site.get_origin()).unwrap_or(0.0));
                    // TODO: Better API
                    this.npcs.spawn(Npc::at(wpos));
                    println!("Spawned rtsim NPC at {:?}", wpos);
                }
                _ => {},
            }
        }

        this
    }
}
