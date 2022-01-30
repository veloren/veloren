#![allow(dead_code)] // TODO: Remove this when rtsim is fleshed out

use super::*;
use crate::sys::terrain::NpcData;
use common::{
    comp,
    event::{EventBus, ServerEvent},
    generation::{BodyBuilder, EntityConfig, EntityInfo},
    resources::{DeltaTime, Time},
    terrain::TerrainGrid,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Join, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::sync::Arc;

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, Time>,
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

    const NAME: &'static str = "rtsim::tick";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            time,
            _dt,
            server_event_bus,
            mut rtsim,
            terrain,
            world,
            index,
            positions,
            rtsim_entities,
            mut agents,
        ): Self::SystemData,
    ) {
        let rtsim = &mut *rtsim;
        rtsim.tick += 1;

        // Update unloaded rtsim entities, in groups at a time
        const TICK_STAGGER: usize = 30;
        let entities_per_iteration = rtsim.entities.len() / TICK_STAGGER;
        let mut to_reify = Vec::new();
        for (id, entity) in rtsim
            .entities
            .iter_mut()
            .skip((rtsim.tick as usize % TICK_STAGGER) * entities_per_iteration)
            .take(entities_per_iteration)
            .filter(|(_, e)| !e.is_loaded)
        {
            // Calculating dt ourselves because the dt provided to this fn was since the
            // last frame, not since the last iteration that these entities acted
            let dt = (time.0 - entity.last_time_ticked) as f32;
            entity.last_time_ticked = time.0;

            if rtsim
                .chunks
                .chunk_at(entity.pos.xy())
                .map(|c| c.is_loaded)
                .unwrap_or(false)
            {
                to_reify.push(id);
            } else {
                // Simulate behaviour
                if let Some(travel_to) = &entity.controller.travel_to {
                    // Move towards target at approximate character speed
                    entity.pos += Vec3::from(
                        (travel_to.0.xy() - entity.pos.xy())
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * entity.get_body().max_speed_approx()
                            * entity.controller.speed_factor,
                    ) * dt;
                }

                if let Some(alt) = world
                    .sim()
                    .get_alt_approx(entity.pos.xy().map(|e| e.floor() as i32))
                {
                    entity.pos.z = alt;
                }
            }
            entity.tick(&time, &terrain, &world, &index.as_index_ref());
        }

        // Tick entity AI each time if it's loaded
        for (_, entity) in rtsim.entities.iter_mut().filter(|(_, e)| e.is_loaded) {
            entity.last_time_ticked = time.0;
            entity.tick(&time, &terrain, &world, &index.as_index_ref());
        }

        let mut server_emitter = server_event_bus.emitter();
        for id in to_reify {
            rtsim.reify_entity(id);
            let entity = &rtsim.entities[id];
            let rtsim_entity = Some(RtSimEntity(id));

            let body = entity.get_body();
            let spawn_pos = terrain
                .find_space(entity.pos.map(|e| e.floor() as i32))
                .map(|e| e as f32)
                + Vec3::new(0.5, 0.5, body.flying_height());

            let pos = comp::Pos(spawn_pos);

            let event = if let comp::Body::Ship(ship) = body {
                ServerEvent::CreateShip {
                    pos,
                    ship,
                    mountable: false,
                    agent: Some(comp::Agent::from_body(&body)),
                    rtsim_entity,
                }
            } else {
                let entity_config_path = entity.get_entity_config();
                let mut loadout_rng = entity.loadout_rng();
                let ad_hoc_loadout = entity.get_adhoc_loadout();
                // Body is rewritten so that body parameters
                // are consistent between reifications
                let entity_config = EntityConfig::from_asset_expect_owned(entity_config_path)
                    .with_body(BodyBuilder::Exact(body));

                let mut entity_info = EntityInfo::at(pos.0)
                    .with_entity_config(entity_config, Some(entity_config_path), &mut loadout_rng)
                    .with_lazy_loadout(ad_hoc_loadout);
                // Merchants can be traded with
                if let Some(economy) = entity.get_trade_info(&world, &index) {
                    entity_info = entity_info
                        .with_agent_mark(comp::agent::Mark::Merchant)
                        .with_economy(&economy);
                }
                match NpcData::from_entity_info(entity_info) {
                    NpcData::Data {
                        pos,
                        stats,
                        skill_set,
                        health,
                        poise,
                        inventory,
                        agent,
                        body,
                        alignment,
                        scale,
                        loot,
                    } => ServerEvent::CreateNpc {
                        pos,
                        stats,
                        skill_set,
                        health,
                        poise,
                        inventory,
                        agent,
                        body,
                        alignment,
                        scale,
                        anchor: None,
                        loot,
                        rtsim_entity,
                        projectile: None,
                    },
                    // EntityConfig can't represent Waypoints at all
                    // as of now, and if someone will try to spawn
                    // rtsim waypoint it is definitely error.
                    NpcData::Waypoint(_) => unimplemented!(),
                }
            };
            server_emitter.emit(event);
        }

        // Update rtsim with real entity data
        for (pos, rtsim_entity, agent) in (&positions, &rtsim_entities, &mut agents).join() {
            rtsim
                .entities
                .get_mut(rtsim_entity.0)
                .filter(|e| e.is_loaded)
                .map(|entity| {
                    entity.pos = pos.0;
                    agent.rtsim_controller = entity.controller.clone();
                });
        }
    }
}
