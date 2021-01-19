#![allow(dead_code)] // TODO: Remove this when rtsim is fleshed out

use super::*;
use common::{
    comp,
    comp::inventory::loadout_builder::LoadoutBuilder,
    event::{EventBus, ServerEvent},
    resources::DeltaTime,
    terrain::TerrainGrid,
};
use specs::{Join, Read, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};
use std::sync::Arc;

const ENTITY_TICK_PERIOD: u64 = 30;

pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        WriteExpect<'a, RtSim>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, Arc<world::World>>,
        ReadExpect<'a, world::IndexOwned>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, RtSimEntity>,
        WriteStorage<'a, comp::Agent>,
    );

    fn run(
        &mut self,
        (
            dt,
            server_event_bus,
            mut rtsim,
            terrain,
            world,
            _index,
            positions,
            rtsim_entities,
            mut agents,
        ): Self::SystemData,
    ) {
        let rtsim = &mut *rtsim;
        rtsim.tick += 1;

        // Update rtsim entities
        // TODO: don't update all of them each tick
        let mut to_reify = Vec::new();
        for (id, entity) in rtsim.entities.iter_mut() {
            if entity.is_loaded {
                // No load-specific behaviour yet
            } else if rtsim
                .chunks
                .chunk_at(entity.pos.xy())
                .map(|c| c.is_loaded)
                .unwrap_or(false)
            {
                to_reify.push(id);
            } else {
                // Simulate behaviour
                if let Some(travel_to) = entity.controller.travel_to {
                    // Move towards target at approximate character speed
                    entity.pos += Vec3::from(
                        (travel_to.xy() - entity.pos.xy())
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * entity.get_body().max_speed_approx()
                            * entity.controller.speed_factor,
                    ) * dt.0;
                }

                if let Some(alt) = world
                    .sim()
                    .get_alt_approx(entity.pos.xy().map(|e| e.floor() as i32))
                {
                    entity.pos.z = alt;
                }
            }

            // Tick entity AI
            if entity.last_tick + ENTITY_TICK_PERIOD <= rtsim.tick {
                entity.tick(&terrain, &world);
                entity.last_tick = rtsim.tick;
            }
        }

        let mut server_emitter = server_event_bus.emitter();
        for id in to_reify {
            rtsim.reify_entity(id);
            let entity = &rtsim.entities[id];
            let spawn_pos = terrain
                .find_space(entity.pos.map(|e| e.floor() as i32))
                .map(|e| e as f32)
                + Vec3::new(0.5, 0.5, 0.0);
            let body = entity.get_body();
            server_emitter.emit(ServerEvent::CreateNpc {
                pos: comp::Pos(spawn_pos),
                stats: comp::Stats::new(entity.get_name(), body),
                health: comp::Health::new(body, 10),
                loadout: match body {
                    comp::Body::Humanoid(_) => entity.get_loadout(),
                    _ => LoadoutBuilder::new().build(),
                },
                body,
                agent: Some(comp::Agent::new(
                    None,
                    matches!(body, comp::Body::Humanoid(_)),
                    &body,
                    false,
                )),
                alignment: match body {
                    comp::Body::Humanoid(_) => comp::Alignment::Npc,
                    _ => comp::Alignment::Wild,
                },
                scale: comp::Scale(1.0),
                drop_item: None,
                home_chunk: None,
                rtsim_entity: Some(RtSimEntity(id)),
            });
        }

        // Update rtsim with real entity data
        for (pos, rtsim_entity, agent) in (&positions, &rtsim_entities, &mut agents).join() {
            rtsim.entities.get_mut(rtsim_entity.0).map(|entity| {
                entity.pos = pos.0;
                agent.rtsim_controller = entity.controller.clone();
            });
        }
    }
}
