use super::super::SysTimer;
use crate::{
    alias_validator::AliasValidator,
    character_creator,
    client::Client,
    persistence::character_loader::CharacterLoader,
    presence::Presence,
    streams::{CharacterScreenStream, GeneralStream, GetStream},
    EditableSettings,
};
use common::{
    comp::{ChatType, Player, UnresolvedChatMsg},
    event::{EventBus, ServerEvent},
    msg::{ClientGeneral, ServerGeneral},
    span,
    state::Time,
    sync::Uid,
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteStorage};
use tracing::{debug, warn};

impl Sys {
    #[allow(clippy::too_many_arguments)]
    fn handle_client_character_screen_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        new_chat_msgs: &mut Vec<(Option<specs::Entity>, UnresolvedChatMsg)>,
        entity: specs::Entity,
        client: &mut Client,
        character_screen_stream: &mut CharacterScreenStream,
        general_stream: &mut GeneralStream,
        character_loader: &ReadExpect<'_, CharacterLoader>,
        uids: &ReadStorage<'_, Uid>,
        players: &ReadStorage<'_, Player>,
        presences: &ReadStorage<'_, Presence>,
        editable_settings: &ReadExpect<'_, EditableSettings>,
        alias_validator: &ReadExpect<'_, AliasValidator>,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        match msg {
            // Request spectator state
            ClientGeneral::Spectate => {
                if players.contains(entity) {
                    warn!("Spectator mode not yet implemented on server");
                } else {
                    debug!("dropped Spectate msg from unregistered client")
                }
            },
            ClientGeneral::Character(character_id) => {
                if let Some(player) = players.get(entity) {
                    if presences.contains(entity) {
                        debug!("player already ingame, aborting");
                    } else {
                        // Send a request to load the character's component data from the
                        // DB. Once loaded, persisted components such as stats and inventory
                        // will be inserted for the entity
                        character_loader.load_character_data(
                            entity,
                            player.uuid().to_string(),
                            character_id,
                        );

                        // Start inserting non-persisted/default components for the entity
                        // while we load the DB data
                        server_emitter.emit(ServerEvent::InitCharacterData {
                            entity,
                            character_id,
                        });

                        // Give the player a welcome message
                        if !editable_settings.server_description.is_empty() {
                            general_stream.send(ChatType::CommandInfo.server_msg(String::from(
                                &*editable_settings.server_description,
                            )))?;
                        }

                        if !client.login_msg_sent {
                            if let Some(player_uid) = uids.get(entity) {
                                new_chat_msgs.push((None, UnresolvedChatMsg {
                                    chat_type: ChatType::Online(*player_uid),
                                    message: "".to_string(),
                                }));

                                client.login_msg_sent = true;
                            }
                        }
                    }
                } else {
                    debug!("Client is not yet registered");
                    character_screen_stream.send(ServerGeneral::CharacterDataLoadError(
                        String::from("Failed to fetch player entity"),
                    ))?
                }
            },
            ClientGeneral::RequestCharacterList => {
                if let Some(player) = players.get(entity) {
                    character_loader.load_character_list(entity, player.uuid().to_string())
                }
            },
            ClientGeneral::CreateCharacter { alias, tool, body } => {
                if let Err(error) = alias_validator.validate(&alias) {
                    debug!(?error, ?alias, "denied alias as it contained a banned word");
                    character_screen_stream
                        .send(ServerGeneral::CharacterActionError(error.to_string()))?;
                } else if let Some(player) = players.get(entity) {
                    character_creator::create_character(
                        entity,
                        player.uuid().to_string(),
                        alias,
                        tool,
                        body,
                        character_loader,
                    );
                }
            },
            ClientGeneral::DeleteCharacter(character_id) => {
                if let Some(player) = players.get(entity) {
                    character_loader.delete_character(
                        entity,
                        player.uuid().to_string(),
                        character_id,
                    );
                }
            },
            _ => unreachable!("not a client_character_screen msg"),
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
        ReadExpect<'a, CharacterLoader>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, Client>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Presence>,
        WriteStorage<'a, CharacterScreenStream>,
        WriteStorage<'a, GeneralStream>,
        ReadExpect<'a, EditableSettings>,
        ReadExpect<'a, AliasValidator>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_event_bus,
            time,
            character_loader,
            mut timer,
            uids,
            mut clients,
            players,
            presences,
            mut character_screen_streams,
            mut general_streams,
            editable_settings,
            alias_validator,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "msg::character_screen::Sys::run");
        timer.start();

        let mut server_emitter = server_event_bus.emitter();
        let mut new_chat_msgs = Vec::new();

        for (entity, client, character_screen_stream, general_stream) in (
            &entities,
            &mut clients,
            &mut character_screen_streams,
            &mut general_streams,
        )
            .join()
        {
            let res =
                super::try_recv_all(character_screen_stream, |character_screen_stream, msg| {
                    Self::handle_client_character_screen_msg(
                        &mut server_emitter,
                        &mut new_chat_msgs,
                        entity,
                        client,
                        character_screen_stream,
                        general_stream,
                        &character_loader,
                        &uids,
                        &players,
                        &presences,
                        &editable_settings,
                        &alias_validator,
                        msg,
                    )
                });

            if let Ok(1_u64..=u64::MAX) = res {
                // Update client ping.
                client.last_ping = time.0
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
