use crate::client::Client;
use common::{
    comp::{ChatMode, ChatType, Content, Group, Player},
    event::{self, EmitExt},
    event_emitters,
    resources::ProgramTime,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use rayon::prelude::*;
use specs::{Entities, LendJoin, ParJoin, Read, ReadStorage, WriteStorage};
use tracing::{debug, error, warn};

event_emitters! {
    struct Events[Emitters] {
        command: event::CommandEvent,
        client_disconnect: event::ClientDisconnectEvent,
        chat: event::ChatEvent,

    }
}

impl Sys {
    fn handle_general_msg(
        emitters: &mut Emitters,
        entity: specs::Entity,
        client: &Client,
        player: Option<&Player>,
        uids: &ReadStorage<'_, Uid>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        groups: &ReadStorage<'_, Group>,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        match msg {
            ClientGeneral::ChatMsg(message) => {
                if player.is_some() {
                    if let Some(from) = uids.get(entity) {
                        const CHAT_MODE_DEFAULT: &ChatMode = &ChatMode::default();
                        let mode = chat_modes.get(entity).unwrap_or(CHAT_MODE_DEFAULT);
                        // Try sending the chat message
                        match mode.to_msg(
                            *from,
                            Content::Plain(message),
                            groups.get(entity).copied(),
                        ) {
                            Ok(message) => {
                                emitters.emit(event::ChatEvent(message));
                            },
                            Err(error) => {
                                client.send_fallible(ServerGeneral::ChatMsg(
                                    ChatType::CommandError.into_msg(error),
                                ));
                            },
                        }
                    } else {
                        error!("Could not send message. Missing player uid");
                    }
                } else {
                    warn!("Received a chat message from an unregistered client");
                }
            },
            ClientGeneral::Command(name, args) => {
                if player.is_some() {
                    emitters.emit(event::CommandEvent(entity, name, args));
                }
            },
            ClientGeneral::Terminate => {
                debug!(?entity, "Client send message to terminate session");
                emitters.emit(event::ClientDisconnectEvent(
                    entity,
                    common::comp::DisconnectReason::ClientRequested,
                ));
            },
            _ => {
                debug!("Kicking possible misbehaving client due to invalid message request");
                emitters.emit(event::ClientDisconnectEvent(
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
        Events<'a>,
        Read<'a, ProgramTime>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, ChatMode>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Group>,
        WriteStorage<'a, Client>,
    );

    const NAME: &'static str = "msg::general";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, events, program_time, uids, chat_modes, players, groups, mut clients): Self::SystemData,
    ) {
        (&entities, &mut clients, players.maybe())
            .par_join()
            .for_each_init(
                || events.get_emitters(),
                |emitters, (entity, client, player)| {
                    let res = super::try_recv_all(client, 3, |client, msg| {
                        Self::handle_general_msg(
                            emitters,
                            entity,
                            client,
                            player,
                            &uids,
                            &chat_modes,
                            &groups,
                            msg,
                        )
                    });

                    if let Ok(1_u64..=u64::MAX) = res {
                        // Update client ping.
                        client.last_ping = program_time.0
                    }
                },
            );
    }
}
