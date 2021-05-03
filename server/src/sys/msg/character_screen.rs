use crate::{
    alias_validator::AliasValidator,
    character_creator,
    client::Client,
    persistence::{character_loader::CharacterLoader, character_updater::CharacterUpdater},
    presence::Presence,
    EditableSettings,
};
use common::{
    comp::{ChatType, Player, UnresolvedChatMsg},
    event::{EventBus, ServerEvent},
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, WriteExpect};
use std::sync::atomic::Ordering;
use tracing::{debug, warn};

impl Sys {
    #[allow(clippy::too_many_arguments)]
    fn handle_client_character_screen_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        entity: specs::Entity,
        client: &Client,
        character_loader: &ReadExpect<'_, CharacterLoader>,
        character_updater: &mut WriteExpect<'_, CharacterUpdater>,
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
                    } else if character_updater
                        .characters_pending_logout()
                        .any(|x| x == character_id)
                    {
                        debug!("player recently logged out pending persistence, aborting");
                        client.send(ServerGeneral::CharacterDataLoadError(
                            "You have recently logged out, please wait a few seconds and try again"
                                .to_string(),
                        ))?;
                    } else if character_updater.disconnect_all_clients_requested() {
                        // If we're in the middle of disconnecting all clients due to a persistence
                        // transaction failure, prevent new logins
                        // temporarily.
                        debug!(
                            "Rejecting player login while pending disconnection of all players is \
                             in progress"
                        );
                        client.send(ServerGeneral::CharacterDataLoadError(
                            "The server is currently recovering from an error, please wait a few \
                             seconds and try again"
                                .to_string(),
                        ))?;
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
                            client.send(ServerGeneral::server_msg(
                                ChatType::CommandInfo,
                                &*editable_settings.server_description,
                            ))?;
                        }

                        if !client.login_msg_sent.load(Ordering::Relaxed) {
                            if let Some(player_uid) = uids.get(entity) {
                                server_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg {
                                    chat_type: ChatType::Online(*player_uid),
                                    message: "".to_string(),
                                }));

                                client.login_msg_sent.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                } else {
                    debug!("Client is not yet registered");
                    client.send(ServerGeneral::CharacterDataLoadError(String::from(
                        "Failed to fetch player entity",
                    )))?
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
                    client.send(ServerGeneral::CharacterActionError(error.to_string()))?;
                } else if let Some(player) = players.get(entity) {
                    character_creator::create_character(
                        entity,
                        player.uuid().to_string(),
                        alias,
                        tool,
                        body,
                        character_updater,
                    );
                }
            },
            ClientGeneral::DeleteCharacter(character_id) => {
                if let Some(player) = players.get(entity) {
                    character_updater.delete_character(
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
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        ReadExpect<'a, CharacterLoader>,
        WriteExpect<'a, CharacterUpdater>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Presence>,
        ReadExpect<'a, EditableSettings>,
        ReadExpect<'a, AliasValidator>,
    );

    const NAME: &'static str = "msg::character_screen";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            server_event_bus,
            character_loader,
            mut character_updater,
            uids,
            clients,
            players,
            presences,
            editable_settings,
            alias_validator,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_event_bus.emitter();

        for (entity, client) in (&entities, &clients).join() {
            let _ = super::try_recv_all(client, 1, |client, msg| {
                Self::handle_client_character_screen_msg(
                    &mut server_emitter,
                    entity,
                    client,
                    &character_loader,
                    &mut character_updater,
                    &uids,
                    &players,
                    &presences,
                    &editable_settings,
                    &alias_validator,
                    msg,
                )
            });
        }
    }
}
