use crate::{client::Client, metrics::PlayerMetrics};
use common::{
    comp::{ChatMode, Player},
    event::{EventBus, ServerEvent},
    resources::Time,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{
    validate_chat_msg, ChatMsgValidationError, ClientGeneral, MAX_BYTES_CHAT_MSG,
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage};
use std::sync::atomic::Ordering;
use tracing::{debug, error, warn};

impl Sys {
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::unnecessary_wraps)]
    fn handle_general_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
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
                            if let Some(message) = message.strip_prefix('/') {
                                if !message.is_empty() {
                                    let argv = String::from(message);
                                    server_emitter.emit(ServerEvent::ChatCmd(entity, argv));
                                }
                            } else if let Some(from) = uids.get(entity) {
                                const CHAT_MODE_DEFAULT: &ChatMode = &ChatMode::default();
                                let mode = chat_modes.get(entity).unwrap_or(CHAT_MODE_DEFAULT);
                                // Send chat message
                                server_emitter
                                    .emit(ServerEvent::Chat(mode.new_message(*from, message)));
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
                debug!(?entity, "Client send message to terminate session");
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
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        ReadExpect<'a, PlayerMetrics>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, ChatMode>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "msg::general";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            server_event_bus,
            time,
            player_metrics,
            uids,
            chat_modes,
            players,
            clients,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_event_bus.emitter();

        for (entity, client, player) in (&entities, &clients, (&players).maybe()).join() {
            let res = super::try_recv_all(client, 3, |client, msg| {
                Self::handle_general_msg(
                    &mut server_emitter,
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
    }
}
