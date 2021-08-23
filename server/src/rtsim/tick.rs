#![allow(dead_code)] // TODO: Remove this when rtsim is fleshed out

use super::*;
use common::{
    comp::{self, inventory::loadout_builder::LoadoutBuilder, Behavior, BehaviorCapability},
    event::{EventBus, ServerEvent},
    resources::{DeltaTime, Time},
    terrain::TerrainGrid,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Join, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::sync::Arc;

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
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
            let body = entity.get_body();
            let spawn_pos = terrain
                .find_space(entity.pos.map(|e| e.floor() as i32))
                .map(|e| e as f32)
                + Vec3::new(0.5, 0.5, body.flying_height());
            let pos = comp::Pos(spawn_pos);
            let agent = Some(comp::Agent::from_body(&body).with_behavior(
                if matches!(body, comp::Body::Humanoid(_)) {
                    Behavior::from(BehaviorCapability::SPEAK)
                } else {
                    Behavior::default()
                },
            ));

            let rtsim_entity = Some(RtSimEntity(id));

            // TODO: this should be a bit more intelligent
            let loadout = match body {
                comp::Body::Humanoid(_) => entity.get_loadout(),
                _ => LoadoutBuilder::empty().with_default_maintool(&body).build(),
            };

            let event = match body {
                comp::Body::Ship(ship) => ServerEvent::CreateShip {
                    pos,
                    ship,
                    mountable: false,
                    agent,
                    rtsim_entity,
                },
                _ => ServerEvent::CreateNpc {
                    pos: comp::Pos(spawn_pos),
                    stats: comp::Stats::new(entity.get_name()),
                    skill_set: comp::SkillSet::default(),
                    health: Some(comp::Health::new(body, 10)),
                    loadout,
                    poise: comp::Poise::new(body),
                    body,
                    agent,
                    alignment: match body {
                        comp::Body::Humanoid(_) => comp::Alignment::Npc,
                        comp::Body::BirdLarge(bird_large) => match bird_large.species {
                            comp::bird_large::Species::Roc => comp::Alignment::Enemy,
                            comp::bird_large::Species::Cockatrice => comp::Alignment::Enemy,
                            _ => comp::Alignment::Wild,
                        },
                        _ => comp::Alignment::Wild,
                    },
                    scale: match body {
                        comp::Body::Ship(_) => comp::Scale(comp::ship::AIRSHIP_SCALE),
                        _ => comp::Scale(1.0),
                    },
                    drop_item: None,
                    anchor: None,
                    rtsim_entity,
                    projectile: None,
                },
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
