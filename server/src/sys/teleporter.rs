use common::{
    comp::{object, Agent, Body, CharacterState, Player, Pos, Teleporter, Teleporting},
    event::{EventBus, ServerEvent},
    resources::Time,
    CachedSpatialGrid,
};
use common_ecs::{Origin, Phase, System};
use specs::{storage::StorageEntry, Entities, Join, Read, ReadStorage, WriteStorage};
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
        ReadStorage<'a, Player>,
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
            players,
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

        for (portal_entity, teleporter_pos, body, teleporter) in
            (&entities, &positions, &bodies, &teleporters).join()
        {
            let nearby_entities = spatial_grid
                .0
                .in_circle_aabr(teleporter_pos.0.xy(), TELEPORT_RADIUS);

            let mut is_active = false;

            let mut player_data = (
                &entities,
                &positions,
                &players,
                &character_states,
                teleporting.entries(),
            )
                .join();

            for (entity, pos, _, character_state, teleporting) in
                nearby_entities.filter_map(|entity| {
                    player_data
                        .get(entity, &entities)
                        .filter(|(_, player_pos, _, _, _)| {
                            in_portal_range(player_pos.0, teleporter_pos.0)
                        })
                })
            {
                if !matches!(
                    character_state,
                    CharacterState::Idle(_) | CharacterState::Wielding(_)
                ) || (teleporter.requires_no_aggro && check_aggro(entity, pos.0))
                {
                    if let StorageEntry::Occupied(entry) = teleporting {
                        entry.remove();
                    };

                    continue;
                }

                if let StorageEntry::Vacant(entry) = teleporting {
                    entry.insert(Teleporting {
                        teleport_start: *time,
                        portal: portal_entity,
                        end_time: Time(time.0 + teleporter.buildup_time.0),
                    });
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

        for (entity, position, _, teleporting_entry) in
            (&entities, &positions, &players, teleporting.entries()).join()
        {
            let StorageEntry::Occupied(teleporting) = teleporting_entry else { continue };
            let portal_pos = positions.get(teleporting.get().portal);
            let Some(teleporter) = teleporters.get(teleporting.get().portal) else {
                teleporting.remove();
                continue
            };

            if portal_pos.map_or(true, |portal_pos| {
                !in_portal_range(position.0, portal_pos.0)
            }) {
                teleporting.remove();
            } else if teleporting.get().end_time.0 <= time.0 {
                teleporting.remove();
                event_bus.emit_now(ServerEvent::TeleportToPosition {
                    entity,
                    position: teleporter.target,
                });
            }
        }
    }
}
