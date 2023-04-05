use crate::{
    event::{EventCtx, OnDeath, OnSetup, OnTick},
    RtState, Rule, RuleError,
};
use common::{grid::Grid, terrain::TerrainChunkSize, vol::RectVolSize};

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

    // Add NPCs to home population (TODO: Do this on entity creation?)
    for (npc_id, npc) in data.npcs.npcs.iter() {
        if let Some(home) = npc.home.and_then(|home| data.sites.get_mut(home)) {
            home.population.insert(npc_id);
        }
    }
}

fn on_death(ctx: EventCtx<SyncNpcs, OnDeath>) {
    let data = &mut *ctx.state.data_mut();

    // Remove NPC from home population
    if let Some(home) = data
        .npcs
        .get(ctx.event.npc_id)
        .and_then(|npc| npc.home)
        .and_then(|home| data.sites.get_mut(home))
    {
        home.population.remove(&ctx.event.npc_id);
    }
}

fn on_tick(ctx: EventCtx<SyncNpcs, OnTick>) {
    let data = &mut *ctx.state.data_mut();
    // Update vehicle grid cells
    for (vehicle_id, vehicle) in data.npcs.vehicles.iter_mut() {
        let chunk_pos = vehicle.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_::<i32>();
        if vehicle.chunk_pos != Some(chunk_pos) {
            if let Some(cell) = vehicle
                .chunk_pos
                .and_then(|chunk_pos| data.npcs.npc_grid.get_mut(chunk_pos))
            {
                if let Some(index) = cell.vehicles.iter().position(|id| *id == vehicle_id) {
                    cell.vehicles.swap_remove(index);
                }
            }
            vehicle.chunk_pos = Some(chunk_pos);
            if let Some(cell) = data.npcs.npc_grid.get_mut(chunk_pos) {
                cell.vehicles.push(vehicle_id);
            }
        }
    }
    for (npc_id, npc) in data.npcs.npcs.iter_mut() {
        // Update the NPC's current site, if any
        npc.current_site = ctx
            .world
            .sim()
            .get(
                npc.wpos
                    .xy()
                    .as_::<i32>()
                    .map2(TerrainChunkSize::RECT_SIZE.as_::<i32>(), |e, sz| {
                        e.div_euclid(sz)
                    }),
            )
            .and_then(|chunk| {
                chunk
                    .sites
                    .iter()
                    .find_map(|site| data.sites.world_site_map.get(site).copied())
            });

        // Update the NPC's grid cell
        let chunk_pos = npc
            .wpos
            .xy()
            .as_::<i32>()
            .map2(TerrainChunkSize::RECT_SIZE.as_::<i32>(), |e, sz| {
                e.div_euclid(sz)
            });
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
