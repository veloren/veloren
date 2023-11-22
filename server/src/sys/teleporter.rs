use common::{
    comp::{Agent, Alignment, CharacterState, Object, Pos, Teleporting},
    consts::TELEPORTER_RADIUS,
    event::{EventBus, TeleportToPositionEvent},
    outcome::Outcome,
    resources::Time,
    uid::Uid,
    CachedSpatialGrid,
};
use common_ecs::{Origin, Phase, System};
use specs::{Entities, Join, LendJoin, Read, ReadStorage, WriteStorage};
use vek::Vec3;

const MAX_AGGRO_DIST: f32 = 200.; // If an entity further than this is aggroed at a player, the portal will still work
const PET_TELEPORT_RADIUS: f32 = 20.;

#[derive(Default)]
pub struct Sys;

fn in_portal_range(player_pos: Vec3<f32>, portal_pos: Vec3<f32>) -> bool {
    player_pos.distance_squared(portal_pos) <= TELEPORTER_RADIUS.powi(2)
}

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Alignment>,
        ReadStorage<'a, Agent>,
        ReadStorage<'a, Object>,
        WriteStorage<'a, Teleporting>,
        ReadStorage<'a, CharacterState>,
        Read<'a, CachedSpatialGrid>,
        Read<'a, Time>,
        Read<'a, EventBus<TeleportToPositionEvent>>,
        Read<'a, EventBus<Outcome>>,
    );

    const NAME: &'static str = "teleporter";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (
            entities,
            positions,
            uids,
            alignments,
            agent,
            objects,
            mut teleporting,
            character_states,
            spatial_grid,
            time,
            teleport_to_position_events,
            outcome_bus,
        ): Self::SystemData,
    ) {
        let mut teleport_to_position_emitter = teleport_to_position_events.emitter();
        let mut outcome_emitter = outcome_bus.emitter();
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

        let mut cancel_teleporting = Vec::new();

        for (entity, uid, position, teleporting, character_state) in (
            &entities,
            &uids,
            &positions,
            &teleporting,
            &character_states,
        )
            .join()
        {
            let portal_pos = positions.get(teleporting.portal);
            let Some(Object::Portal {
                target,
                requires_no_aggro,
                ..
            }) = objects.get(teleporting.portal)
            else {
                cancel_teleporting.push(entity);
                continue;
            };

            if portal_pos.map_or(true, |portal_pos| {
                !in_portal_range(position.0, portal_pos.0)
            }) || (*requires_no_aggro && check_aggro(entity, position.0))
                || !matches!(
                    character_state,
                    CharacterState::Idle(_)
                        | CharacterState::Wielding(_)
                        | CharacterState::Sit
                        | CharacterState::Dance
                )
            {
                cancel_teleporting.push(entity);
            } else if teleporting.end_time.0 <= time.0 {
                // Send teleport events for all nearby pets and the owner
                let nearby = spatial_grid
                    .0
                    .in_circle_aabr(position.0.xy(), PET_TELEPORT_RADIUS)
                    .filter_map(|entity| {
                        (&entities, &positions, &alignments)
                            .lend_join()
                            .get(entity, &entities)
                    })
                    .filter_map(|(nearby_entity, entity_position, alignment)| {
                        (matches!(alignment, Alignment::Owned(entity_uid) if entity_uid == uid)
                            && entity_position.0.distance_squared(position.0)
                                <= PET_TELEPORT_RADIUS.powi(2)
                            // Allow for non-players to teleport too
                            || entity == nearby_entity)
                            .then_some(nearby_entity)
                    });

                for entity in nearby {
                    cancel_teleporting.push(entity);
                    teleport_to_position_emitter.emit(TeleportToPositionEvent {
                        entity,
                        position: *target,
                    });
                    outcome_emitter.emit(Outcome::TeleportedByPortal { pos: *target });
                }
            }
        }

        for entity in cancel_teleporting {
            let _ = teleporting.remove(entity);
        }
    }
}
