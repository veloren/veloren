use crate::{
    automod::AutoMod,
    character_creator,
    client::Client,
    persistence::{character_loader::CharacterLoader, character_updater::CharacterUpdater},
    presence::Presence,
    EditableSettings,
};
use common::{
    comp::{Admin, AdminRole, ChatType, Player, UnresolvedChatMsg},
    event::{EventBus, ServerEvent},
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::sync::{atomic::Ordering, Arc};
use tracing::debug;

impl Sys {
    fn handle_client_character_screen_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        entity: specs::Entity,
        client: &Client,
        character_loader: &ReadExpect<'_, CharacterLoader>,
        character_updater: &mut WriteExpect<'_, CharacterUpdater>,
        uids: &ReadStorage<'_, Uid>,
        players: &ReadStorage<'_, Player>,
        admins: &ReadStorage<'_, Admin>,
        presences: &ReadStorage<'_, Presence>,
        editable_settings: &ReadExpect<'_, EditableSettings>,
        censor: &ReadExpect<'_, Arc<censor::Censor>>,
        automod: &AutoMod,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        let mut send_join_messages = || -> Result<(), crate::error::Error> {
            // Give the player a welcome message
            if !editable_settings.server_description.is_empty() {
                client.send(ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    &*editable_settings.server_description,
                ))?;
            }

            // Warn them about automod
            if automod.enabled() {
                client.send(ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    "Automatic moderation is enabled: play nice and have fun!",
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
            Ok(())
        };
        match msg {
            // Request spectator state
            ClientGeneral::Spectate(requested_view_distances) => {
                if let Some(admin) = admins.get(entity) && admin.0 >= AdminRole::Moderator {
                    send_join_messages()?;

                    server_emitter.emit(ServerEvent::InitSpectator(entity, requested_view_distances));

                } else {
                    debug!("dropped Spectate msg from unprivileged client")
                }
            },
            ClientGeneral::Character(character_id, requested_view_distances) => {
                if let Some(player) = players.get(entity) {
                    if presences.contains(entity) {
                        debug!("player already ingame, aborting");
                    } else if character_updater.has_pending_database_action(character_id)
                    {
                        debug!("player recently logged out pending persistence, aborting");
                        client.send(ServerGeneral::CharacterDataLoadResult(Err(
                            "You have recently logged out, please wait a few seconds and try again"
                                .to_string(),
                        )))?;
                    } else if character_updater.disconnect_all_clients_requested() {
                        // If we're in the middle of disconnecting all clients due to a persistence
                        // transaction failure, prevent new logins
                        // temporarily.
                        debug!(
                            "Rejecting player login while pending disconnection of all players is \
                             in progress"
                        );
                        client.send(ServerGeneral::CharacterDataLoadResult(Err(
                            "The server is currently recovering from an error, please wait a few \
                             seconds and try again"
                                .to_string(),
                        )))?;
                    } else {
                        // Send a request to load the character's component data from the
                        // DB. Once loaded, persisted components such as stats and inventory
                        // will be inserted for the entity
                        character_loader.load_character_data(
                            entity,
                            player.uuid().to_string(),
                            character_id,
                        );

                        send_join_messages()?;

                        // Start inserting non-persisted/default components for the entity
                        // while we load the DB data
                        server_emitter.emit(ServerEvent::InitCharacterData {
                            entity,
                            character_id,
                            requested_view_distances,
                        });
                    }
                } else {
                    debug!("Client is not yet registered");
                    client.send(ServerGeneral::CharacterDataLoadResult(Err(String::from(
                        "Failed to fetch player entity",
                    ))))?
                }
            },
            ClientGeneral::RequestCharacterList => {
                if let Some(player) = players.get(entity) {
                    character_loader.load_character_list(entity, player.uuid().to_string())
                }
            },
            ClientGeneral::CreateCharacter {
                alias,
                mainhand,
                offhand,
                body,
            } => {
                if censor.check(&alias) {
                    debug!(?alias, "denied alias as it contained a banned word");
                    client.send(ServerGeneral::CharacterActionError(format!(
                        "Alias '{}' contains a banned word",
                        alias
                    )))?;
                } else if let Some(player) = players.get(entity) {
                    if let Err(error) = character_creator::create_character(
                        entity,
                        player.uuid().to_string(),
                        alias,
                        mainhand.clone(),
                        offhand.clone(),
                        body,
                        character_updater,
                    ) {
                        debug!(
                            ?error,
                            ?mainhand,
                            ?offhand,
                            ?body,
                            "Denied creating character because of invalid input."
                        );
                        client.send(ServerGeneral::CharacterActionError(error.to_string()))?;
                    }
                }
            },
            ClientGeneral::EditCharacter { id, alias, body } => {
                if censor.check(&alias) {
                    debug!(?alias, "denied alias as it contained a banned word");
                    client.send(ServerGeneral::CharacterActionError(format!(
                        "Alias '{}' contains a banned word",
                        alias
                    )))?;
                } else if let Some(player) = players.get(entity) {
                    if let Err(error) = character_creator::edit_character(
                        entity,
                        player.uuid().to_string(),
                        id,
                        alias,
                        body,
                        character_updater,
                    ) {
                        debug!(
                            ?error,
                            ?body,
                            "Denied editing character because of invalid input."
                        );
                        client.send(ServerGeneral::CharacterActionError(error.to_string()))?;
                    }
                }
            },
            ClientGeneral::DeleteCharacter(character_id) => {
                if let Some(player) = players.get(entity) {
                    server_emitter.emit(ServerEvent::DeleteCharacter {
                        entity,
                        requesting_player_uuid: player.uuid().to_string(),
                        character_id,
                    });
                }
            },
            _ => {
                debug!("Kicking possibly misbehaving client due to invalid character request");
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
        ReadExpect<'a, CharacterLoader>,
        WriteExpect<'a, CharacterUpdater>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, Client>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Admin>,
        ReadStorage<'a, Presence>,
        ReadExpect<'a, EditableSettings>,
        ReadExpect<'a, Arc<censor::Censor>>,
        ReadExpect<'a, AutoMod>,
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
            mut clients,
            players,
            admins,
            presences,
            editable_settings,
            censor,
            automod,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_event_bus.emitter();

        for (entity, client) in (&entities, &mut clients).join() {
            let _ = super::try_recv_all(client, 1, |client, msg| {
                Self::handle_client_character_screen_msg(
                    &mut server_emitter,
                    entity,
                    client,
                    &character_loader,
                    &mut character_updater,
                    &uids,
                    &players,
                    &admins,
                    &presences,
                    &editable_settings,
                    &censor,
                    &automod,
                    msg,
                )
            });
        }
    }
}
