use crate::client::Client;
use common::{
    comp::{ChatMode, Player},
    event::{EventBus, ServerEvent},
    resources::Time,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::ClientGeneral;
use rayon::prelude::*;
use specs::{Entities, Join, ParJoin, Read, ReadStorage, WriteStorage};
use tracing::{debug, error, warn};

impl Sys {
    fn handle_general_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        entity: specs::Entity,
        _client: &Client,
        player: Option<&Player>,
        uids: &ReadStorage<'_, Uid>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        match msg {
            ClientGeneral::ChatMsg(message) => {
                if player.is_some() {
                    if let Some(from) = uids.get(entity) {
                        const CHAT_MODE_DEFAULT: &ChatMode = &ChatMode::default();
                        let mode = chat_modes.get(entity).unwrap_or(CHAT_MODE_DEFAULT);
                        // Send chat message
                        server_emitter.emit(ServerEvent::Chat(mode.new_message(*from, message)));
                    } else {
                        error!("Could not send message. Missing player uid");
                    }
                } else {
                    warn!("Received a chat message from an unregistered client");
                }
            },
            ClientGeneral::Command(name, args) => {
                if player.is_some() {
                    server_emitter.emit(ServerEvent::Command(entity, name, args));
                }
            },
            ClientGeneral::Terminate => {
                debug!(?entity, "Client send message to terminate session");
                server_emitter.emit(ServerEvent::ClientDisconnect(
                    entity,
                    common::comp::DisconnectReason::ClientRequested,
                ));
            },
            _ => {
                debug!("Kicking possible misbehaving client due to invalid message request");
                server_emitter.emit(ServerEvent::ClientDisconnect(
                    entity,
                    common::comp::DisconnectReason::NetworkError,
                ));
            },
        }
        Ok(())
    }
}

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, ChatMode>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, Client>,
    );

    const NAME: &'static str = "msg::general";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, server_event_bus, time, uids, chat_modes, players, mut clients): Self::SystemData,
    ) {
        (&entities, &mut clients, players.maybe())
            .par_join()
            .for_each_init(
                || server_event_bus.emitter(),
                |server_emitter, (entity, client, player)| {
                    let res = super::try_recv_all(client, 3, |client, msg| {
                        Self::handle_general_msg(
                            server_emitter,
                            entity,
                            client,
                            player,
                            &uids,
                            &chat_modes,
                            msg,
                        )
                    });

                    if let Ok(1_u64..=u64::MAX) = res {
                        // Update client ping.
                        client.last_ping = time.0
                    }
                },
            );
    }
}
