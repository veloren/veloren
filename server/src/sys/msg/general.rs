use super::super::SysTimer;
use crate::{client::Client, metrics::PlayerMetrics};
use common::{
    comp::{ChatMode, Player, UnresolvedChatMsg},
    event::{EventBus, ServerEvent},
    resources::Time,
    span,
    uid::Uid,
};
use common_net::msg::{
    validate_chat_msg, ChatMsgValidationError, ClientGeneral, MAX_BYTES_CHAT_MSG,
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, Write};
use std::sync::atomic::Ordering;
use tracing::{debug, error, warn};

impl Sys {
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::unnecessary_wraps)]
    fn handle_general_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        new_chat_msgs: &mut Vec<(Option<specs::Entity>, UnresolvedChatMsg)>,
        entity: specs::Entity,
        client: &Client,
        player: Option<&Player>,
        player_metrics: &ReadExpect<'_, PlayerMetrics>,
        uids: &ReadStorage<'_, Uid>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        match msg {
            ClientGeneral::ChatMsg(message) => {
                if player.is_some() {
                    match validate_chat_msg(&message) {
                        Ok(()) => {
                            if let Some(from) = uids.get(entity) {
                                let mode = chat_modes.get(entity).cloned().unwrap_or_default();
                                let msg = mode.new_message(*from, message);
                                new_chat_msgs.push((Some(entity), msg));
                            } else {
                                error!("Could not send message. Missing player uid");
                            }
                        },
                        Err(ChatMsgValidationError::TooLong) => {
                            let max = MAX_BYTES_CHAT_MSG;
                            let len = message.len();
                            warn!(?len, ?max, "Received a chat message that's too long")
                        },
                    }
                }
            },
            ClientGeneral::Terminate => {
                debug!(?entity, "Client send message to termitate session");
                player_metrics
                    .clients_disconnected
                    .with_label_values(&["gracefully"])
                    .inc();
                client.terminate_msg_recv.store(true, Ordering::Relaxed);
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
            },
            _ => unreachable!("not a client_general msg"),
        }
        Ok(())
    }
}

/// This system will handle new messages from clients
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        ReadExpect<'a, PlayerMetrics>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, ChatMode>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Client>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_event_bus,
            time,
            player_metrics,
            mut timer,
            uids,
            chat_modes,
            players,
            clients,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "msg::general::Sys::run");
        timer.start();

        let mut server_emitter = server_event_bus.emitter();
        let mut new_chat_msgs = Vec::new();

        for (entity, client, player) in (&entities, &clients, (&players).maybe()).join() {
            let res = super::try_recv_all(client, 3, |client, msg| {
                Self::handle_general_msg(
                    &mut server_emitter,
                    &mut new_chat_msgs,
                    entity,
                    client,
                    player,
                    &player_metrics,
                    &uids,
                    &chat_modes,
                    msg,
                )
            });

            if let Ok(1_u64..=u64::MAX) = res {
                // Update client ping.
                *client.last_ping.lock().unwrap() = time.0
            }
        }

        // Handle new chat messages.
        for (entity, msg) in new_chat_msgs {
            // Handle chat commands.
            if msg.message.starts_with('/') {
                if let (Some(entity), true) = (entity, msg.message.len() > 1) {
                    let argv = String::from(&msg.message[1..]);
                    server_emitter.emit(ServerEvent::ChatCmd(entity, argv));
                }
            } else {
                // Send chat message
                server_emitter.emit(ServerEvent::Chat(msg));
            }
        }

        timer.end()
    }
}
