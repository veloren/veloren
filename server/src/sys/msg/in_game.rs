use crate::{client::Client, presence::Presence, Settings};
use common::{
    comp::{
        CanBuild, ControlEvent, Controller, ForceUpdate, Health, Ori, Player, Pos, SkillSet, Vel,
    },
    event::{EventBus, ServerEvent},
    resources::PlayerPhysicsSettings,
    terrain::TerrainGrid,
    vol::ReadVol,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, PresenceKind, ServerGeneral};
use common_sys::state::{BlockChange, BuildAreas};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, Write, WriteStorage};
use tracing::{debug, trace, warn};

impl Sys {
    #[allow(clippy::too_many_arguments)]
    fn handle_client_in_game_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        entity: specs::Entity,
        client: &Client,
        maybe_presence: &mut Option<&mut Presence>,
        terrain: &ReadExpect<'_, TerrainGrid>,
        can_build: &ReadStorage<'_, CanBuild>,
        force_updates: &ReadStorage<'_, ForceUpdate>,
        skill_sets: &mut WriteStorage<'_, SkillSet>,
        healths: &ReadStorage<'_, Health>,
        block_changes: &mut Write<'_, BlockChange>,
        positions: &mut WriteStorage<'_, Pos>,
        velocities: &mut WriteStorage<'_, Vel>,
        orientations: &mut WriteStorage<'_, Ori>,
        controllers: &mut WriteStorage<'_, Controller>,
        settings: &Read<'_, Settings>,
        build_areas: &Read<'_, BuildAreas>,
        player_physics_settings: &mut Write<'_, PlayerPhysicsSettings>,
        maybe_player: &Option<&Player>,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        let presence = match maybe_presence {
            Some(g) => g,
            None => {
                debug!(?entity, "client is not in_game, ignoring msg");
                trace!(?msg, "ignored msg content");
                return Ok(());
            },
        };
        match msg {
            // Go back to registered state (char selection screen)
            ClientGeneral::ExitInGame => {
                server_emitter.emit(ServerEvent::ExitIngame { entity });
                client.send(ServerGeneral::ExitInGameSuccess)?;
                *maybe_presence = None;
            },
            ClientGeneral::SetViewDistance(view_distance) => {
                presence.view_distance = settings
                    .max_view_distance
                    .map(|max| view_distance.min(max))
                    .unwrap_or(view_distance);

                //correct client if its VD is to high
                if settings
                    .max_view_distance
                    .map(|max| view_distance > max)
                    .unwrap_or(false)
                {
                    client.send(ServerGeneral::SetViewDistance(
                        settings.max_view_distance.unwrap_or(0),
                    ))?;
                }
            },
            ClientGeneral::ControllerInputs(inputs) => {
                if matches!(presence.kind, PresenceKind::Character(_)) {
                    if let Some(controller) = controllers.get_mut(entity) {
                        controller.inputs.update_with_new(*inputs);
                    }
                }
            },
            ClientGeneral::ControlEvent(event) => {
                if matches!(presence.kind, PresenceKind::Character(_)) {
                    // Skip respawn if client entity is alive
                    if let ControlEvent::Respawn = event {
                        if healths.get(entity).map_or(true, |h| !h.is_dead) {
                            //Todo: comment why return!
                            return Ok(());
                        }
                    }
                    if let Some(controller) = controllers.get_mut(entity) {
                        controller.events.push(event);
                    }
                }
            },
            ClientGeneral::ControlAction(event) => {
                if matches!(presence.kind, PresenceKind::Character(_)) {
                    if let Some(controller) = controllers.get_mut(entity) {
                        controller.actions.push(event);
                    }
                }
            },
            ClientGeneral::PlayerPhysics { pos, vel, ori } => {
                let player_physics_setting = maybe_player.map(|p| {
                    player_physics_settings
                        .settings
                        .entry(p.uuid())
                        .or_default()
                });
                if matches!(presence.kind, PresenceKind::Character(_))
                    && force_updates.get(entity).is_none()
                    && healths.get(entity).map_or(true, |h| !h.is_dead)
                    && player_physics_setting
                        .as_ref()
                        .map_or(true, |s| s.client_authoritative())
                {
                    let mut reject_update = false;
                    if let Some(mut setting) = player_physics_setting {
                        // If we detect any thresholds being exceeded, force server-authoritative
                        // physics for that player. This doesn't detect subtle hacks, but it
                        // prevents blatent ones and forces people to not debug physics hacks on the
                        // live server (and also mitigates some floating-point overflow crashes)
                        if let Some(prev_pos) = positions.get(entity) {
                            let value_squared = prev_pos.0.distance_squared(pos.0);
                            if value_squared > (5000.0f32).powf(2.0) {
                                setting.server_force = true;
                                reject_update = true;
                                warn!(
                                    "PlayerPhysics position exceeded {:?} {:?} {:?}",
                                    prev_pos,
                                    pos,
                                    value_squared.sqrt()
                                );
                            }
                        }

                        if vel.0.magnitude_squared() > (500.0f32).powf(2.0) {
                            setting.server_force = true;
                            reject_update = true;
                            warn!(
                                "PlayerPhysics velocity exceeded {:?} {:?}",
                                pos,
                                vel.0.magnitude()
                            );
                        }
                    }

                    if reject_update {
                        warn!(
                            "Rejected PlayerPhysics update {:?} {:?} {:?} {:?}",
                            pos, vel, ori, maybe_player
                        );
                    } else {
                        let _ = positions.insert(entity, pos);
                        let _ = velocities.insert(entity, vel);
                        let _ = orientations.insert(entity, ori);
                    }
                }
            },
            ClientGeneral::BreakBlock(pos) => {
                if let Some(comp_can_build) = can_build.get(entity) {
                    if comp_can_build.enabled {
                        for area in comp_can_build.build_areas.iter() {
                            if let Some(block) = build_areas
                                .areas()
                                .get(*area)
                                // TODO: Make this an exclusive check on the upper bound of the AABB
                                // Vek defaults to inclusive which is not optimal
                                .filter(|aabb| aabb.contains_point(pos))
                                .and_then(|_| terrain.get(pos).ok())
                            {
                                block_changes.set(pos, block.into_vacant());
                            }
                        }
                    }
                }
            },
            ClientGeneral::PlaceBlock(pos, block) => {
                if let Some(comp_can_build) = can_build.get(entity) {
                    if comp_can_build.enabled {
                        for area in comp_can_build.build_areas.iter() {
                            if build_areas
                                .areas()
                                .get(*area)
                                // TODO: Make this an exclusive check on the upper bound of the AABB
                                // Vek defaults to inclusive which is not optimal
                                .filter(|aabb| aabb.contains_point(pos))
                                .is_some()
                            {
                                block_changes.try_set(pos, block);
                            }
                        }
                    }
                }
            },
            ClientGeneral::UnlockSkill(skill) => {
                skill_sets
                    .get_mut(entity)
                    .map(|mut skill_set| skill_set.unlock_skill(skill));
            },
            ClientGeneral::RefundSkill(skill) => {
                skill_sets
                    .get_mut(entity)
                    .map(|mut skill_set| skill_set.refund_skill(skill));
            },
            ClientGeneral::UnlockSkillGroup(skill_group_kind) => {
                skill_sets
                    .get_mut(entity)
                    .map(|mut skill_set| skill_set.unlock_skill_group(skill_group_kind));
            },
            ClientGeneral::RequestSiteInfo(id) => {
                server_emitter.emit(ServerEvent::RequestSiteInfo { entity, id });
            },
            ClientGeneral::RequestPlayerPhysics {
                server_authoritative,
            } => {
                let player_physics_setting = maybe_player.map(|p| {
                    player_physics_settings
                        .settings
                        .entry(p.uuid())
                        .or_default()
                });
                if let Some(setting) = player_physics_setting {
                    setting.client_optin = server_authoritative;
                }
            },
            _ => tracing::error!("not a client_in_game msg"),
        }
        Ok(())
    }
}

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        ReadExpect<'a, TerrainGrid>,
        ReadStorage<'a, CanBuild>,
        ReadStorage<'a, ForceUpdate>,
        WriteStorage<'a, SkillSet>,
        ReadStorage<'a, Health>,
        Write<'a, BlockChange>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Presence>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, Controller>,
        Read<'a, Settings>,
        Read<'a, BuildAreas>,
        Write<'a, PlayerPhysicsSettings>,
        ReadStorage<'a, Player>,
    );

    const NAME: &'static str = "msg::in_game";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            server_event_bus,
            terrain,
            can_build,
            force_updates,
            mut skill_sets,
            healths,
            mut block_changes,
            mut positions,
            mut velocities,
            mut orientations,
            mut presences,
            mut clients,
            mut controllers,
            settings,
            build_areas,
            mut player_physics_settings,
            players,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_event_bus.emitter();

        for (entity, client, mut maybe_presence, player) in (
            &entities,
            &mut clients,
            (&mut presences).maybe(),
            players.maybe(),
        )
            .join()
        {
            let _ = super::try_recv_all(client, 2, |client, msg| {
                Self::handle_client_in_game_msg(
                    &mut server_emitter,
                    entity,
                    client,
                    &mut maybe_presence.as_deref_mut(),
                    &terrain,
                    &can_build,
                    &force_updates,
                    &mut skill_sets,
                    &healths,
                    &mut block_changes,
                    &mut positions,
                    &mut velocities,
                    &mut orientations,
                    &mut controllers,
                    &settings,
                    &build_areas,
                    &mut player_physics_settings,
                    &player,
                    msg,
                )
            });
        }
    }
}
