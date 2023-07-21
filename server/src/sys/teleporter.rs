use std::time::Duration;

use common::{
    comp::{
        ability::AbilityMeta, object, Agent, Body, CharacterState, ForceUpdate, Player, Pos,
        Teleporter, Teleporting,
    },
    resources::Time,
    states::{
        blink,
        utils::{AbilityInfo, StageSection},
    },
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
        WriteStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Teleporter>,
        ReadStorage<'a, Agent>,
        WriteStorage<'a, ForceUpdate>,
        WriteStorage<'a, Teleporting>,
        WriteStorage<'a, Body>,
        WriteStorage<'a, CharacterState>,
        Read<'a, CachedSpatialGrid>,
        Read<'a, Time>,
    );

    const NAME: &'static str = "teleporter";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (
            entities,
            mut positions,
            players,
            teleporters,
            agent,
            mut forced_update,
            mut teleporting,
            mut bodies,
            mut character_states,
            spatial_grid,
            time,
        ): Self::SystemData,
    ) {
        let mut attempt_teleport = vec![];
        let mut cancel_teleport = vec![];

        let mut player_data = (
            &entities,
            &positions,
            &players,
            &mut character_states,
            teleporting.entries(),
        )
            .join();

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

        for (portal_entity, teleporter_pos, mut body, teleporter) in
            (&entities, &positions, &mut bodies, &teleporters).join()
        {
            let nearby_entities = spatial_grid
                .0
                .in_circle_aabr(teleporter_pos.0.xy(), TELEPORT_RADIUS);

            let mut is_active = false;

            for (entity, pos, _, mut character_state, teleporting) in
                nearby_entities.filter_map(|entity| {
                    player_data
                        .get(entity, &entities)
                        .filter(|(_, player_pos, _, _, _)| {
                            in_portal_range(player_pos.0, teleporter_pos.0)
                        })
                })
            {
                if teleporter.requires_no_aggro && check_aggro(entity, pos.0) {
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
                } else if let Some(remaining) = if let StorageEntry::Occupied(entry) = teleporting {
                    let teleporting = entry.get();
                    ((time.0 - teleporting.teleport_start.0) >= teleporter.buildup_time.0 / 3.
                        && !matches!(*character_state, CharacterState::Blink(_)))
                    .then_some(teleporter.buildup_time.0 - (time.0 - teleporting.teleport_start.0))
                } else {
                    None
                } {
                    // Move into blink character state at half buildup time
                    *character_state = CharacterState::Blink(blink::Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Buildup,
                        static_data: blink::StaticData {
                            buildup_duration: Duration::from_secs_f64(remaining),
                            recover_duration: Duration::default(),
                            max_range: 0.,
                            ability_info: AbilityInfo {
                                tool: None,
                                hand: None,
                                input: common::comp::InputKind::Primary,
                                ability_meta: AbilityMeta::default(),
                                ability: None,
                                input_attr: None,
                            },
                        },
                    });
                }

                is_active = true;
            }

            if (*body == Body::Object(object::Body::PortalActive)) != is_active {
                *body = Body::Object(if is_active {
                    object::Body::PortalActive
                } else {
                    object::Body::Portal
                });
            }
        }

        for (entity, position, _, teleporting) in
            (&entities, &positions, &players, &teleporting).join()
        {
            let portal_pos = positions.get(teleporting.portal);
            let Some(teleporter) = teleporters.get(teleporting.portal) else {
                cancel_teleport.push(entity);
                continue
            };

            if portal_pos.map_or(true, |portal_pos| {
                !in_portal_range(position.0, portal_pos.0)
            }) {
                cancel_teleport.push(entity);
            } else if teleporting.end_time.0 <= time.0 {
                attempt_teleport.push((entity, *teleporter));
                cancel_teleport.push(entity);
            }
        }

        for entity in cancel_teleport {
            teleporting.remove(entity);
            character_states.get_mut(entity).map(|mut state| {
                if let CharacterState::Blink(data) = &mut *state {
                    data.stage_section = StageSection::Recover;
                }
            });
        }

        for (entity, teleporter) in attempt_teleport {
            positions
                .get_mut(entity)
                .map(|position| position.0 = teleporter.target);
            forced_update
                .get_mut(entity)
                .map(|forced_update| forced_update.update());
        }
    }
}
