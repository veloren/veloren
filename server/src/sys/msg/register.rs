use crate::{
    client::Client,
    login_provider::{LoginProvider, PendingLogin},
    metrics::PlayerMetrics,
    EditableSettings, Settings,
};
use common::{
    comp::{Admin, Player, Stats},
    event::{EventBus, ServerEvent},
    uid::{Uid, UidAllocator},
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{
    CharacterInfo, ClientRegister, DisconnectReason, PlayerInfo, PlayerListUpdate, RegisterError,
    ServerGeneral, ServerRegisterAnswer,
};
use hashbrown::HashMap;
use plugin_api::Health;
use specs::{
    shred::ResourceId, storage::StorageEntry, Entities, Join, Read, ReadExpect, ReadStorage,
    SystemData, World, WriteExpect, WriteStorage,
};
use tracing::trace;

#[cfg(feature = "plugins")]
use {common_state::plugin::memory_manager::EcsWorld, common_state::plugin::PluginMgr};

#[cfg(feature = "plugins")]
type ReadPlugin<'a> = Read<'a, PluginMgr>;
#[cfg(not(feature = "plugins"))]
type ReadPlugin<'a> = Option<Read<'a, ()>>;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    stats: ReadStorage<'a, Stats>,
    uids: ReadStorage<'a, Uid>,
    clients: ReadStorage<'a, Client>,
    server_event_bus: Read<'a, EventBus<ServerEvent>>,
    player_metrics: ReadExpect<'a, PlayerMetrics>,
    settings: ReadExpect<'a, Settings>,
    editable_settings: ReadExpect<'a, EditableSettings>,
    _healths: ReadStorage<'a, Health>, // used by plugin feature
    _plugin_mgr: ReadPlugin<'a>,       // used by plugin feature
    _uid_allocator: Read<'a, UidAllocator>, // used by plugin feature
}

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, Admin>,
        WriteStorage<'a, PendingLogin>,
        WriteExpect<'a, LoginProvider>,
    );

    const NAME: &'static str = "msg::register";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            read_data,
            mut players,
            mut admins,
            mut pending_logins,
            mut login_provider,
        ): Self::SystemData,
    ) {
        let mut server_emitter = read_data.server_event_bus.emitter();
        // Player list to send new players.
        let player_list = (
            &read_data.uids,
            &players,
            read_data.stats.maybe(),
            admins.maybe(),
        )
            .join()
            .map(|(uid, player, stats, admin)| {
                (*uid, PlayerInfo {
                    is_online: true,
                    is_moderator: admin.is_some(),
                    player_alias: player.alias.clone(),
                    character: stats.map(|stats| CharacterInfo {
                        name: stats.name.clone(),
                    }),
                })
            })
            .collect::<HashMap<_, _>>();
        // List of new players to update player lists of all clients.
        let mut new_players = Vec::new();

        // defer auth lockup
        for (entity, client) in (&read_data.entities, &read_data.clients).join() {
            let _ = super::try_recv_all(client, 0, |_, msg: ClientRegister| {
                trace!(?msg.token_or_username, "defer auth lockup");
                let pending = login_provider.verify(&msg.token_or_username);
                let _ = pending_logins.insert(entity, pending);
                Ok(())
            });
        }

        let mut finished_pending = vec![];
        let mut retries = vec![];
        for (entity, client, mut pending) in
            (&read_data.entities, &read_data.clients, &mut pending_logins).join()
        {
            if let Err(e) = || -> std::result::Result<(), crate::error::Error> {
                #[cfg(feature = "plugins")]
                let ecs_world = EcsWorld {
                    entities: &read_data.entities,
                    health: (&read_data._healths).into(),
                    uid: (&read_data.uids).into(),
                    player: (&players).into(),
                    uid_allocator: &read_data._uid_allocator,
                };

                let (username, uuid) = match login_provider.login(
                    &mut pending,
                    #[cfg(feature = "plugins")]
                    &ecs_world,
                    #[cfg(feature = "plugins")]
                    &read_data._plugin_mgr,
                    &*read_data.editable_settings.admins,
                    &*read_data.editable_settings.whitelist,
                    &*read_data.editable_settings.banlist,
                ) {
                    None => return Ok(()),
                    Some(r) => {
                        finished_pending.push(entity);
                        trace!(?r, "pending login returned");
                        match r {
                            Err(e) => {
                                server_emitter.emit(ServerEvent::ClientDisconnect(
                                    entity,
                                    common::comp::DisconnectReason::Kicked,
                                ));
                                client.send(ServerRegisterAnswer::Err(e))?;
                                return Ok(());
                            },
                            Ok((username, uuid)) => (username, uuid),
                        }
                    },
                };

                // Check if user is already logged-in
                if let Some((old_entity, old_client, _)) =
                    (&read_data.entities, &read_data.clients, &players)
                        .join()
                        .find(|(_, _, old_player)| old_player.uuid() == uuid)
                {
                    // Remove old client
                    server_emitter.emit(ServerEvent::ClientDisconnect(
                        old_entity,
                        common::comp::DisconnectReason::NewerLogin,
                    ));
                    let _ = old_client.send(ServerGeneral::Disconnect(DisconnectReason::Kicked(
                        String::from("You have logged in from another location."),
                    )));
                    // We can't login the new client right now as the
                    // removal of the old client and player occurs later in
                    // the tick, so we instead setup the new login to be
                    // processed in the next tick
                    // Create "fake" successful pending auth and mark it to
                    // be inserted into pending_logins at the end of this
                    // run
                    retries.push((entity, PendingLogin::new_success(username, uuid)));
                    return Ok(());
                }

                let player = Player::new(username, read_data.settings.battle_mode, uuid);
                let admin = read_data.editable_settings.admins.get(&uuid);

                if !player.is_valid() {
                    // Invalid player
                    client.send(ServerRegisterAnswer::Err(RegisterError::InvalidCharacter))?;
                    return Ok(());
                }

                if let Ok(StorageEntry::Vacant(v)) = players.entry(entity) {
                    // Add Player component to this client, if the entity exists.
                    v.insert(player);
                    read_data.player_metrics.players_connected.inc();

                    // Give the Admin component to the player if their name exists in
                    // admin list
                    if let Some(admin) = admin {
                        admins
                            .insert(entity, Admin(admin.role.into()))
                            .expect("Inserting into players proves the entity exists.");
                    }

                    // Tell the client its request was successful.
                    client.send(ServerRegisterAnswer::Ok(()))?;

                    // Send initial player list
                    client.send(ServerGeneral::PlayerListUpdate(PlayerListUpdate::Init(
                        player_list.clone(),
                    )))?;

                    // Add to list to notify all clients of the new player
                    new_players.push(entity);
                }
                Ok(())
            }() {
                tracing::trace!(?e, "failed to process register")
            };
        }
        for e in finished_pending {
            pending_logins.remove(e);
        }
        // Insert retry attempts back into pending_logins to be processed next tick
        for (entity, pending) in retries {
            let _ = pending_logins.insert(entity, pending);
        }

        // Handle new players.
        // Tell all clients to add them to the player list.
        let player_info = |entity| {
            let player_info = read_data.uids.get(entity).zip(players.get(entity));
            player_info.map(|(u, p)| (entity, u, p))
        };
        for (entity, uid, player) in new_players.into_iter().filter_map(player_info) {
            let mut lazy_msg = None;
            for (_, client) in (&players, &read_data.clients).join() {
                if lazy_msg.is_none() {
                    lazy_msg = Some(client.prepare(ServerGeneral::PlayerListUpdate(
                        PlayerListUpdate::Add(*uid, PlayerInfo {
                            player_alias: player.alias.clone(),
                            is_online: true,
                            is_moderator: admins.get(entity).is_some(),
                            character: None, // new players will be on character select.
                        }),
                    )));
                }
                lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
            }
        }
    }
}
