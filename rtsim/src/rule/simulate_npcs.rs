use crate::{
    data::npc::SimulationMode,
    event::{OnSetup, OnTick},
    RtState, Rule, RuleError,
};
use common::{grid::Grid, terrain::TerrainChunkSize, vol::RectVolSize};
use tracing::info;
use vek::*;

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnSetup>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            data.npcs.npc_grid = Grid::new(ctx.world.sim().get_size().as_(), Default::default());

            for (npc_id, npc) in data.npcs.npcs.iter() {
                if let Some(ride) = &npc.riding {
                    if let Some(vehicle) = data.npcs.vehicles.get_mut(ride.vehicle) {
                        let actor = crate::data::Actor::Npc(npc_id);
                        vehicle.riders.push(actor);
                        if ride.steering {
                            if vehicle.driver.replace(actor).is_some() {
                                panic!("Replaced driver");
                            }
                        }
                    }
                }
            }
        });
        rtstate.bind::<Self, OnTick>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            for (vehicle_id, vehicle) in data.npcs.vehicles.iter_mut() {
                let chunk_pos =
                    vehicle.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_::<i32>();
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
                    .get(npc.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_())
                    .and_then(|chunk| data.sites.world_site_map.get(chunk.sites.first()?).copied());

                let chunk_pos =
                    npc.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_::<i32>();
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

                // Simulate the NPC's movement and interactions
                if matches!(npc.mode, SimulationMode::Simulated) {
                    if let Some(riding) = &npc.riding {
                        if let Some(vehicle) = data.npcs.vehicles.get_mut(riding.vehicle) {
                            if let Some(action) = npc.action && riding.steering {
                                match action {
                                    crate::data::npc::NpcAction::Goto(target, speed_factor) => {
                                        let diff = target.xy() - vehicle.wpos.xy();
                                        let dist2 = diff.magnitude_squared();

                                        if dist2 > 0.5f32.powi(2) {
                                            let mut wpos = vehicle.wpos + (diff
                                                * (vehicle.get_speed() * speed_factor * ctx.event.dt
                                                    / dist2.sqrt())
                                                .min(1.0))
                                            .with_z(0.0);

                                            let is_valid = match vehicle.body {
                                                common::comp::ship::Body::DefaultAirship | common::comp::ship::Body::AirBalloon => true,
                                                common::comp::ship::Body::SailBoat | common::comp::ship::Body::Galleon => {
                                                    let chunk_pos = wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_::<i32>();
                                                    ctx.world.sim().get(chunk_pos).map_or(true, |f| f.river.river_kind.is_some())
                                                },
                                                _ => false,
                                            };

                                            if is_valid {
                                                match vehicle.body {
                                                    common::comp::ship::Body::DefaultAirship | common::comp::ship::Body::AirBalloon => {
                                                        if let Some(alt) = ctx.world.sim().get_alt_approx(wpos.xy().as_()).filter(|alt| wpos.z < *alt) {
                                                            wpos.z = alt;
                                                        }
                                                    },
                                                    common::comp::ship::Body::SailBoat | common::comp::ship::Body::Galleon => {
                                                        wpos.z = ctx
                                                            .world
                                                            .sim()
                                                            .get_interpolated(wpos.xy().map(|e| e as i32), |chunk| chunk.water_alt)
                                                            .unwrap_or(0.0);
                                                    },
                                                    _ => {},
                                                }
                                                vehicle.wpos = wpos;
                                            }
                                        }
                                    }
                                }
                            }
                            npc.wpos = vehicle.wpos;
                        } else {
                            // Vehicle doens't exist anymore
                            npc.riding = None;
                        }
                    }
                    // Move NPCs if they have a target destination
                    else if let Some(action) = npc.action {
                        match action {
                            crate::data::npc::NpcAction::Goto(target, speed_factor) => {
                                let diff = target.xy() - npc.wpos.xy();
                                let dist2 = diff.magnitude_squared();

                                if dist2 > 0.5f32.powi(2) {
                                    npc.wpos += (diff
                                        * (npc.body.max_speed_approx() * speed_factor * ctx.event.dt
                                            / dist2.sqrt())
                                        .min(1.0))
                                    .with_z(0.0);
                                }
                            },
                        }

                        // Make sure NPCs remain on the surface
                        npc.wpos.z = ctx
                            .world
                            .sim()
                            .get_surface_alt_approx(npc.wpos.xy().map(|e| e as i32))
                            .unwrap_or(0.0) + npc.body.flying_height();
                    }

                }
            }
        });

        Ok(Self)
    }
}
