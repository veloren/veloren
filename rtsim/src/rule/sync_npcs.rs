use crate::{
    event::{EventCtx, OnDeath, OnSetup, OnTick},
    RtState, Rule, RuleError,
};
use common::{
    grid::Grid,
    rtsim::{Actor, NpcInput},
    terrain::CoordinateConversions,
};

pub struct SyncNpcs;

impl Rule for SyncNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnSetup>(on_setup);
        rtstate.bind::<Self, OnDeath>(on_death);
        rtstate.bind::<Self, OnTick>(on_tick);

        Ok(Self)
    }
}

fn on_setup(ctx: EventCtx<SyncNpcs, OnSetup>) {
    let data = &mut *ctx.state.data_mut();

    // Create NPC grid
    data.npcs.npc_grid = Grid::new(ctx.world.sim().get_size().as_(), Default::default());

    // Add NPCs to home population
    for (npc_id, npc) in data.npcs.npcs.iter() {
        if let Some(home) = npc.home.and_then(|home| data.sites.get_mut(home)) {
            home.population.insert(npc_id);
        }
    }

    // Update the list of nearest sites by size for each site
    let sites_iter = data.sites.iter().filter_map(|(site_id, site)| {
        let site2 = site
            .world_site
            .and_then(|ws| ctx.index.sites.get(ws).site2())?;
        Some((site_id, site, site2))
    });
    let nearest_by_size = sites_iter.clone()
        .map(|(site_id, site, site2)| {
            let mut other_sites = sites_iter.clone()
                // Only include sites in the list if they're not the current one and they're more populus
                .filter(|(other_id, _, other_site2)| *other_id != site_id && other_site2.plots().len() > site2.plots().len())
                .collect::<Vec<_>>();
            other_sites.sort_by_key(|(_, other, _)| other.wpos.as_::<i64>().distance_squared(site.wpos.as_::<i64>()));
            let mut max_size = 0;
            // Remove sites that aren't in increasing order of size (Stalin sort?!)
            other_sites.retain(|(_, _, other_site2)| {
                if other_site2.plots().len() > max_size {
                    max_size = other_site2.plots().len();
                    true
                } else {
                    false
                }
            });
            let nearest_by_size = other_sites
                .into_iter()
                .map(|(site_id, _, _)| site_id)
                .collect::<Vec<_>>();
            (site_id, nearest_by_size)
        })
        .collect::<Vec<_>>();
    for (site_id, nearest_by_size) in nearest_by_size {
        if let Some(site) = data.sites.get_mut(site_id) {
            site.nearby_sites_by_size = nearest_by_size;
        }
    }
}

fn on_death(ctx: EventCtx<SyncNpcs, OnDeath>) {
    let data = &mut *ctx.state.data_mut();

    if let Actor::Npc(npc_id) = ctx.event.actor {
        if let Some(npc) = data.npcs.get_mut(npc_id) {
            // Mark the NPC as dead, allowing us to clear them up later
            npc.is_dead = true;
        }
    }
}

fn on_tick(ctx: EventCtx<SyncNpcs, OnTick>) {
    let data = &mut *ctx.state.data_mut();
    for (npc_id, npc) in data.npcs.npcs.iter_mut() {
        // Update the NPC's current site, if any
        npc.current_site = ctx
            .world
            .sim()
            .get(npc.wpos.xy().as_().wpos_to_cpos())
            .and_then(|chunk| {
                chunk
                    .sites
                    .iter()
                    .find_map(|site| data.sites.world_site_map.get(site).copied())
            });

        // Share known reports with current site, if it's our home
        // TODO: Only share new reports
        if let Some(current_site) = npc.current_site
            && Some(current_site) == npc.home
        {
            if let Some(site) = data.sites.get_mut(current_site) {
                // TODO: Sites should have an inbox and their own AI code
                site.known_reports.extend(npc.known_reports.iter().copied());
                npc.inbox.extend(
                    site.known_reports
                        .iter()
                        .copied()
                        .filter(|report| !npc.known_reports.contains(report))
                        .map(NpcInput::Report),
                );
            }
        }

        // Update the NPC's grid cell
        let chunk_pos = npc.wpos.xy().as_().wpos_to_cpos();
        if npc.chunk_pos != Some(chunk_pos) {
            if let Some(cell) = npc
                .chunk_pos
                .and_then(|chunk_pos| data.npcs.npc_grid.get_mut(chunk_pos))
            {
                if let Some(index) = cell.npcs.iter().position(|id| *id == npc_id) {
                    cell.npcs.swap_remove(index);
                }
            }
            npc.chunk_pos = Some(chunk_pos);
            if let Some(cell) = data.npcs.npc_grid.get_mut(chunk_pos) {
                cell.npcs.push(npc_id);
            }
        }
    }
}
