use super::super::SysTimer;
use crate::{
    client::Client,
    metrics::NetworkRequestMetrics,
    streams::{GetStream, InGameStream},
    Settings,
};
use common::{
    comp::{CanBuild, ControlEvent, Controller, ForceUpdate, Ori, Player, Pos, Stats, Vel},
    event::{EventBus, ServerEvent},
    msg::{ClientGeneral, ClientInGame, ServerGeneral},
    span,
    state::{BlockChange, Time},
    terrain::{TerrainChunkSize, TerrainGrid},
    vol::{ReadVol, RectVolSize},
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteStorage};
use tracing::{debug, trace};

impl Sys {
    #[allow(clippy::too_many_arguments)]
    fn handle_client_in_game_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        entity: specs::Entity,
        client: &mut Client,
        in_game_stream: &mut InGameStream,
        terrain: &ReadExpect<'_, TerrainGrid>,
        network_metrics: &ReadExpect<'_, NetworkRequestMetrics>,
        can_build: &ReadStorage<'_, CanBuild>,
        force_updates: &ReadStorage<'_, ForceUpdate>,
        stats: &mut WriteStorage<'_, Stats>,
        block_changes: &mut Write<'_, BlockChange>,
        positions: &mut WriteStorage<'_, Pos>,
        velocities: &mut WriteStorage<'_, Vel>,
        orientations: &mut WriteStorage<'_, Ori>,
        players: &mut WriteStorage<'_, Player>,
        controllers: &mut WriteStorage<'_, Controller>,
        settings: &Read<'_, Settings>,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        if client.in_game.is_none() {
            debug!(?entity, "client is not in_game, ignoring msg");
            trace!(?msg, "ignored msg content");
            if matches!(msg, ClientGeneral::TerrainChunkRequest{ .. }) {
                network_metrics.chunks_request_dropped.inc();
            }
            return Ok(());
        }
        match msg {
            // Go back to registered state (char selection screen)
            ClientGeneral::ExitInGame => {
                client.in_game = None;
                server_emitter.emit(ServerEvent::ExitIngame { entity });
                in_game_stream.send(ServerGeneral::ExitInGameSuccess)?;
            },
            ClientGeneral::SetViewDistance(view_distance) => {
                players.get_mut(entity).map(|player| {
                    player.view_distance = Some(
                        settings
                            .max_view_distance
                            .map(|max| view_distance.min(max))
                            .unwrap_or(view_distance),
                    )
                });

                //correct client if its VD is to high
                if settings
                    .max_view_distance
                    .map(|max| view_distance > max)
                    .unwrap_or(false)
                {
                    in_game_stream.send(ServerGeneral::SetViewDistance(
                        settings.max_view_distance.unwrap_or(0),
                    ))?;
                }
            },
            ClientGeneral::ControllerInputs(inputs) => {
                if let Some(ClientInGame::Character) = client.in_game {
                    if let Some(controller) = controllers.get_mut(entity) {
                        controller.inputs.update_with_new(inputs);
                    }
                }
            },
            ClientGeneral::ControlEvent(event) => {
                if let Some(ClientInGame::Character) = client.in_game {
                    // Skip respawn if client entity is alive
                    if let ControlEvent::Respawn = event {
                        if stats.get(entity).map_or(true, |s| !s.is_dead) {
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
                if let Some(ClientInGame::Character) = client.in_game {
                    if let Some(controller) = controllers.get_mut(entity) {
                        controller.actions.push(event);
                    }
                }
            },
            ClientGeneral::PlayerPhysics { pos, vel, ori } => {
                if let Some(ClientInGame::Character) = client.in_game {
                    if force_updates.get(entity).is_none()
                        && stats.get(entity).map_or(true, |s| !s.is_dead)
                    {
                        let _ = positions.insert(entity, pos);
                        let _ = velocities.insert(entity, vel);
                        let _ = orientations.insert(entity, ori);
                    }
                }
            },
            ClientGeneral::BreakBlock(pos) => {
                if let Some(block) = can_build.get(entity).and_then(|_| terrain.get(pos).ok()) {
                    block_changes.set(pos, block.into_vacant());
                }
            },
            ClientGeneral::PlaceBlock(pos, block) => {
                if can_build.get(entity).is_some() {
                    block_changes.try_set(pos, block);
                }
            },
            ClientGeneral::TerrainChunkRequest { key } => {
                let in_vd = if let (Some(view_distance), Some(pos)) = (
                    players.get(entity).and_then(|p| p.view_distance),
                    positions.get(entity),
                ) {
                    pos.0.xy().map(|e| e as f64).distance(
                        key.map(|e| e as f64 + 0.5) * TerrainChunkSize::RECT_SIZE.map(|e| e as f64),
                    ) < (view_distance as f64 - 1.0 + 2.5 * 2.0_f64.sqrt())
                        * TerrainChunkSize::RECT_SIZE.x as f64
                } else {
                    true
                };
                if in_vd {
                    match terrain.get_key(key) {
                        Some(chunk) => {
                            network_metrics.chunks_served_from_memory.inc();
                            in_game_stream.send(ServerGeneral::TerrainChunkUpdate {
                                key,
                                chunk: Ok(Box::new(chunk.clone())),
                            })?
                        },
                        None => {
                            network_metrics.chunks_generation_triggered.inc();
                            server_emitter.emit(ServerEvent::ChunkRequest(entity, key))
                        },
                    }
                } else {
                    network_metrics.chunks_request_dropped.inc();
                }
            },
            ClientGeneral::UnlockSkill(skill) => {
                stats
                    .get_mut(entity)
                    .map(|s| s.skill_set.unlock_skill(skill));
            },
            ClientGeneral::RefundSkill(skill) => {
                stats
                    .get_mut(entity)
                    .map(|s| s.skill_set.refund_skill(skill));
            },
            ClientGeneral::UnlockSkillGroup(skill_group_type) => {
                stats
                    .get_mut(entity)
                    .map(|s| s.skill_set.unlock_skill_group(skill_group_type));
            },
            _ => unreachable!("not a client_in_game msg"),
        }
        Ok(())
    }
}

/// This system will handle new messages from clients
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, NetworkRequestMetrics>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, CanBuild>,
        ReadStorage<'a, ForceUpdate>,
        WriteStorage<'a, Stats>,
        Write<'a, BlockChange>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, InGameStream>,
        WriteStorage<'a, Controller>,
        Read<'a, Settings>,
    );

    #[allow(clippy::match_ref_pats)] // TODO: Pending review in #587
    #[allow(clippy::single_char_pattern)] // TODO: Pending review in #587
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn run(
        &mut self,
        (
            entities,
            server_event_bus,
            time,
            terrain,
            network_metrics,
            mut timer,
            can_build,
            force_updates,
            mut stats,
            mut block_changes,
            mut positions,
            mut velocities,
            mut orientations,
            mut players,
            mut clients,
            mut in_game_streams,
            mut controllers,
            settings,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "msg::in_game::Sys::run");
        timer.start();

        let mut server_emitter = server_event_bus.emitter();

        for (entity, client, in_game_stream) in
            (&entities, &mut clients, &mut in_game_streams).join()
        {
            let res = super::try_recv_all(in_game_stream, |in_game_stream, msg| {
                Self::handle_client_in_game_msg(
                    &mut server_emitter,
                    entity,
                    client,
                    in_game_stream,
                    &terrain,
                    &network_metrics,
                    &can_build,
                    &force_updates,
                    &mut stats,
                    &mut block_changes,
                    &mut positions,
                    &mut velocities,
                    &mut orientations,
                    &mut players,
                    &mut controllers,
                    &settings,
                    msg,
                )
            });

            match res {
                Ok(1_u64..=u64::MAX) => {
                    // Update client ping.
                    client.last_ping = time.0
                },
                _ => (/*handled by ping*/),
            }
        }

        timer.end()
    }
}
