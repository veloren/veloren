use crate::{
    automod::{self, AutoMod},
    client::Client,
};
use common::{
    comp::{Admin, AdminRole, ChatMode, ChatType, Player},
    event::{EventBus, ServerEvent},
    resources::Time,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use specs::{Entities, Join, Read, ReadStorage, WriteExpect};
use std::time::Instant;
use tracing::{debug, error, warn};

impl Sys {
    fn handle_general_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        entity: specs::Entity,
        client: &Client,
        player: Option<&Player>,
        admin_role: Option<AdminRole>,
        uids: &ReadStorage<'_, Uid>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        msg: ClientGeneral,
        now: Instant,
        automod: &mut AutoMod,
    ) -> Result<(), crate::error::Error> {
        match msg {
            ClientGeneral::ChatMsg(message) => {
                if let Some(player) = player {
                    match automod.validate_chat_msg(player.uuid(), admin_role, now, &message) {
                        Ok(note) => {
                            if let Some(from) = uids.get(entity) {
                                const CHAT_MODE_DEFAULT: &ChatMode = &ChatMode::default();
                                let mode = chat_modes.get(entity).unwrap_or(CHAT_MODE_DEFAULT);
                                // Send chat message
                                server_emitter
                                    .emit(ServerEvent::Chat(mode.new_message(*from, message)));
                            } else {
                                error!("Could not send message. Missing player uid");
                            }

                            match note {
                                None => {},
                                Some(automod::ActionNote::SpamWarn) => {
                                    let _ = client.send(ServerGeneral::server_msg(
                                        ChatType::CommandError,
                                        "You've sent a lot of messages recently. Make sure to \
                                         reduce the rate of messages or you will be automatically \
                                         muted.",
                                    ));
                                },
                            }
                        },
                        Err(automod::ActionErr::TooLong) => {
                            let len = message.len();
                            warn!(?len, "Received a chat message that's too long");
                        },
                        Err(automod::ActionErr::BannedWord) => {
                            let _ = client.send(ServerGeneral::server_msg(
                                ChatType::CommandError,
                                "Your message contained a banned word. If you think this is a \
                                 false positive, please open a bug report.",
                            ));
                        },
                        Err(automod::ActionErr::SpamMuted(dur)) => {
                            let _ = client.send(ServerGeneral::server_msg(
                                ChatType::CommandError,
                                format!(
                                    "You have sent too many messages and are muted for {} seconds.",
                                    dur.as_secs_f32() as u64
                                ),
                            ));
                        },
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
        ReadStorage<'a, Client>,
        ReadStorage<'a, Admin>,
        WriteExpect<'a, AutoMod>,
    );

    const NAME: &'static str = "msg::general";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, server_event_bus, time, uids, chat_modes, players, clients, admins, mut automod): Self::SystemData,
    ) {
        let mut server_emitter = server_event_bus.emitter();

        let now = Instant::now();
        for (entity, client, player, admin) in
            (&entities, &clients, players.maybe(), admins.maybe()).join()
        {
            let res = super::try_recv_all(client, 3, |client, msg| {
                Self::handle_general_msg(
                    &mut server_emitter,
                    entity,
                    client,
                    player,
                    admin.map(|a| a.0),
                    &uids,
                    &chat_modes,
                    msg,
                    now,
                    &mut automod,
                )
            });

            if let Ok(1_u64..=u64::MAX) = res {
                // Update client ping.
                *client.last_ping.lock().unwrap() = time.0
            }
        }
    }
}
