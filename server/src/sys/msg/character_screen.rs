#[cfg(not(feature = "worldgen"))]
use crate::test_world::{IndexOwned, World};
#[cfg(feature = "worldgen")]
use world::{IndexOwned, World};

use crate::{
    automod::AutoMod,
    character_creator,
    client::Client,
    persistence::{character_loader::CharacterLoader, character_updater::CharacterUpdater},
    EditableSettings,
};
use common::{
    comp::{Admin, AdminRole, ChatType, Player, Presence, Waypoint},
    event::{
        ChatEvent, ClientDisconnectEvent, DeleteCharacterEvent, EmitExt, InitializeCharacterEvent,
        InitializeSpectatorEvent,
    },
    event_emitters,
    resources::Time,
    terrain::TerrainChunkSize,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use specs::{Entities, Join, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::sync::{atomic::Ordering, Arc};
use tracing::{debug, error};

event_emitters! {
    struct Events[Emitters] {
        init_spectator: InitializeSpectatorEvent,
        init_character_data: InitializeCharacterEvent,
        delete_character: DeleteCharacterEvent,
        client_disconnect: ClientDisconnectEvent,
        chat: ChatEvent,
    }
}

impl Sys {
    #[allow(clippy::too_many_arguments)] // Shhhh, go bother someone else clippy
    fn handle_client_character_screen_msg(
        emitters: &mut Emitters,
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
        time: Time,
        index: &ReadExpect<'_, IndexOwned>,
        world: &ReadExpect<'_, Arc<World>>,
    ) -> Result<(), crate::error::Error> {
        let mut send_join_messages = || -> Result<(), crate::error::Error> {
            // Give the player a welcome message
            let localized_description = editable_settings
                .server_description
                .get(client.locale.as_deref());
            if !localized_description.map_or(true, |d| d.motd.is_empty()) {
                client.send(ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    localized_description.map_or("", |d| &d.motd),
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
                    emitters.emit(ChatEvent(ChatType::Online(*player_uid).into_plain_msg("")));

                    client.login_msg_sent.store(true, Ordering::Relaxed);
                }
            }
            Ok(())
        };
        match msg {
            // Request spectator state
            ClientGeneral::Spectate(requested_view_distances) => {
                if let Some(admin) = admins.get(entity)
                    && admin.0 >= AdminRole::Moderator
                {
                    send_join_messages()?;

                    emitters.emit(InitializeSpectatorEvent(entity, requested_view_distances));
                } else {
                    debug!("dropped Spectate msg from unprivileged client")
                }
            },
            ClientGeneral::Character(character_id, requested_view_distances) => {
                if let Some(player) = players.get(entity) {
                    // NOTE: Because clients retain their Uid when exiting to the character
                    // selection screen, we rely on this check to prevent them from immediately
                    // re-entering in-game in the same tick so that we can synchronize their
                    // removal without this being mixed up (and e.g. telling other clients to
                    // delete the re-joined version) or requiring more complex handling to avoid
                    // this.
                    if presences.contains(entity) {
                        debug!("player already ingame, aborting");
                    } else if character_updater.has_pending_database_action(character_id) {
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
                        emitters.emit(InitializeCharacterEvent {
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
                start_site,
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
                        start_site.and_then(|site_idx| {
                            // TODO: This corresponds to the ID generation logic in
                            // `world/src/lib.rs`. Really, we should have
                            // a way to consistently refer to sites, but that's a job for rtsim2
                            // and the site changes that it will require. Until then, this code is
                            // very hacky.
                            world
                                .civs()
                                .sites
                                .iter()
                                .find(|(_, site)| site.site_tmp.map(|i| i.id()) == Some(site_idx))
                                .map(Some)
                                .unwrap_or_else(|| {
                                    error!(
                                        "Tried to create character with starting site index {}, \
                                         but such a site does not exist",
                                        site_idx
                                    );
                                    None
                                })
                                .map(|(_, site)| {
                                    let wpos2d = TerrainChunkSize::center_wpos(site.center);
                                    Waypoint::new(
                                        world.find_accessible_pos(
                                            index.as_index_ref(),
                                            wpos2d,
                                            false,
                                        ),
                                        time,
                                    )
                                })
                        }),
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
                    emitters.emit(DeleteCharacterEvent {
                        entity,
                        requesting_player_uuid: player.uuid().to_string(),
                        character_id,
                    });
                }
            },
            _ => {
                debug!("Kicking possibly misbehaving client due to invalid character request");
                emitters.emit(ClientDisconnectEvent(
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
        ReadExpect<'a, Time>,
        ReadExpect<'a, IndexOwned>,
        ReadExpect<'a, Arc<World>>,
    );

    const NAME: &'static str = "msg::character_screen";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            events,
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
            time,
            index,
            world,
        ): Self::SystemData,
    ) {
        let mut emitters = events.get_emitters();

        for (entity, client) in (&entities, &mut clients).join() {
            let _ = super::try_recv_all(client, 1, |client, msg| {
                Self::handle_client_character_screen_msg(
                    &mut emitters,
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
                    *time,
                    &index,
                    &world,
                )
            });
        }
    }
}
