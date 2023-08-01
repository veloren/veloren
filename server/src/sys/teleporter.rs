use common::{
    comp::{object, Agent, Body, CharacterState, Pos, Teleporter, Teleporting},
    event::{EventBus, ServerEvent},
    resources::Time,
    CachedSpatialGrid,
};
use common_ecs::{Origin, Phase, System};
use specs::{Entities, Entity, Join, Read, ReadStorage, WriteStorage};
use vek::Vec3;

const TELEPORT_RADIUS: f32 = 3.;
const MAX_AGGRO_DIST: f32 = 200.; // If an entity further than this is aggroed at a player, the portal will still work

#[derive(Default)]
pub struct Sys;

fn in_portal_range(player_pos: Vec3<f32>, portal_pos: Vec3<f32>) -> bool {
    player_pos.distance_squared(portal_pos) <= (TELEPORT_RADIUS).powi(2)
}

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Teleporter>,
        ReadStorage<'a, Agent>,
        WriteStorage<'a, Teleporting>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, CharacterState>,
        Read<'a, CachedSpatialGrid>,
        Read<'a, Time>,
        Read<'a, EventBus<ServerEvent>>,
    );

    const NAME: &'static str = "teleporter";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (
            entities,
            positions,
            teleporters,
            agent,
            mut teleporting,
            bodies,
            character_states,
            spatial_grid,
            time,
            event_bus,
        ): Self::SystemData,
    ) {
        let check_aggro = |entity, pos: Vec3<f32>| {
            spatial_grid
                .0
                .in_circle_aabr(pos.xy(), MAX_AGGRO_DIST)
                .any(|agent_entity| {
                    agent.get(agent_entity).map_or(false, |agent| {
                        agent.target.map_or(false, |agent_target| {
                            agent_target.target == entity && agent_target.aggro_on
                        })
                    })
                })
        };

        let mut teleporting_updates = Vec::new();

        for (portal_entity, teleporter_pos, body, teleporter) in
            (&entities, &positions, &bodies, &teleporters).join()
        {
            let nearby_entities = spatial_grid
                .0
                .in_circle_aabr(teleporter_pos.0.xy(), TELEPORT_RADIUS);

            let mut is_active = false;

            for (entity, pos, character_state, teleporting) in
                nearby_entities.filter_map(|entity| {
                    (
                        &entities,
                        &positions,
                        &character_states,
                        teleporting.maybe(),
                    )
                        .join()
                        .get(entity, &entities)
                        .filter(|(_, pos, _, _)| in_portal_range(pos.0, teleporter_pos.0))
                })
            {
                if !matches!(character_state, CharacterState::Dance)
                    || (teleporter.requires_no_aggro && check_aggro(entity, pos.0))
                {
                    if teleporting.is_some() {
                        teleporting_updates.push((entity, None));
                    };

                    continue;
                }

                if teleporting.is_none() {
                    teleporting_updates.push((
                        entity,
                        Some(Teleporting {
                            teleport_start: *time,
                            portal: portal_entity,
                            end_time: Time(time.0 + teleporter.buildup_time.0),
                        }),
                    ));
                }

                is_active = true;
            }

            if (*body == Body::Object(object::Body::PortalActive)) != is_active {
                event_bus.emit_now(ServerEvent::ChangeBody {
                    entity: portal_entity,
                    new_body: Body::Object(if is_active {
                        object::Body::PortalActive
                    } else {
                        object::Body::Portal
                    }),
                });
            }
        }

        update_teleporting(&mut teleporting_updates, &mut teleporting);

        for (entity, position, teleporting) in (&entities, &positions, &teleporting).join() {
            let mut remove = || teleporting_updates.push((entity, None));
            let portal_pos = positions.get(teleporting.portal);
            let Some(teleporter) = teleporters.get(teleporting.portal) else {
                remove();
                continue
            };

            if portal_pos.map_or(true, |portal_pos| {
                !in_portal_range(position.0, portal_pos.0)
            }) {
                remove();
            } else if teleporting.end_time.0 <= time.0 {
                remove();
                event_bus.emit_now(ServerEvent::TeleportToPosition {
                    entity,
                    position: teleporter.target,
                });
            }
        }

        update_teleporting(&mut teleporting_updates, &mut teleporting);
    }
}

fn update_teleporting(
    updates: &mut Vec<(Entity, Option<Teleporting>)>,
    teleporting: &mut WriteStorage<'_, Teleporting>,
) {
    for (entity, update) in updates.drain(..) {
        if let Some(add) = update {
            let _ = teleporting.insert(entity, add);
        } else {
            let _ = teleporting.remove(entity);
        }
    }
}
