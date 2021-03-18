use crate::{
    client::Client,
    login_provider::{LoginProvider, PendingLogin},
    metrics::PlayerMetrics,
    EditableSettings,
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
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use tracing::trace;

#[cfg(feature = "plugins")]
use common_sys::plugin::memory_manager::EcsWorld;

#[cfg(feature = "plugins")]
use common_sys::plugin::PluginMgr;

#[cfg(feature = "plugins")]
type ReadPlugin<'a> = Read<'a, PluginMgr>;
#[cfg(not(feature = "plugins"))]
type ReadPlugin<'a> = Option<Read<'a, ()>>;

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, PlayerMetrics>,
        ReadStorage<'a, Health>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Client>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, PendingLogin>,
        Read<'a, UidAllocator>,
        ReadPlugin<'a>,
        ReadStorage<'a, Stats>,
        WriteExpect<'a, LoginProvider>,
        WriteStorage<'a, Admin>,
        ReadExpect<'a, EditableSettings>,
        Read<'a, EventBus<ServerEvent>>,
    );

    const NAME: &'static str = "msg::register";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            player_metrics,
            health_comp,
            uids,
            clients,
            mut players,
            mut pending_logins,
            uid_allocator,
            plugin_mgr,
            stats,
            mut login_provider,
            mut admins,
            editable_settings,
            server_event_bus,
        ): Self::SystemData,
    ) {
        // Player list to send new players.
        let player_list = (&uids, &players, stats.maybe(), admins.maybe())
            .join()
            .map(|(uid, player, stats, admin)| {
                (*uid, PlayerInfo {
                    is_online: true,
                    is_admin: admin.is_some(),
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
        for (entity, client) in (&entities, &clients).join() {
            let _ = super::try_recv_all(client, 0, |_, msg: ClientRegister| {
                trace!(?msg.token_or_username, "defer auth lockup");
                let pending = login_provider.verify(&msg.token_or_username);
                let _ = pending_logins.insert(entity, pending);
                Ok(())
            });
        }

        let mut finished_pending = vec![];
        let mut retries = vec![];
        for (entity, client, mut pending) in (&entities, &clients, &mut pending_logins).join() {
            if let Err(e) = || -> std::result::Result<(), crate::error::Error> {
                #[cfg(feature = "plugins")]
                let ecs_world = EcsWorld {
                    entities: &entities,
                    health: (&health_comp).into(),
                    uid: (&uids).into(),
                    player: (&players).into(),
                    uid_allocator: &uid_allocator,
                };

                let (username, uuid) = match login_provider.try_login(
                    &mut pending,
                    #[cfg(feature = "plugins")]
                    &ecs_world,
                    #[cfg(feature = "plugins")]
                    &plugin_mgr,
                    &*editable_settings.admins,
                    &*editable_settings.whitelist,
                    &*editable_settings.banlist,
                ) {
                    None => return Ok(()),
                    Some(r) => {
                        finished_pending.push(entity);
                        trace!(?r, "pending login returned");
                        match r {
                            Err(e) => {
                                let mut retry = false;
                                if let RegisterError::AlreadyLoggedIn(uuid, ref username) = e {
                                    if let Some((old_entity, old_client, _)) =
                                        (&entities, &clients, &players)
                                            .join()
                                            .find(|(_, _, old_player)| old_player.uuid() == uuid)
                                    {
                                        // Remove old client
                                        server_event_bus
                                            .emit_now(ServerEvent::ClientDisconnect(old_entity));
                                        let _ = old_client.send(ServerGeneral::Disconnect(
                                            DisconnectReason::Kicked(String::from(
                                                "You have logged in from another location.",
                                            )),
                                        ));
                                        // We can't login the new client right now as the
                                        // removal of the old client and player occurs later in
                                        // the tick, so we instead setup the new login to be
                                        // processed in the next tick
                                        // Create "fake" successful pending auth and mark it to
                                        // be inserted into pending_logins at the end of this
                                        // run
                                        retries.push((
                                            entity,
                                            PendingLogin::new_success(username.to_string(), uuid),
                                        ));
                                        retry = true;
                                    }
                                }
                                if !retry {
                                    client.send(ServerRegisterAnswer::Err(e))?;
                                }
                                return Ok(());
                            },
                            Ok((username, uuid)) => (username, uuid),
                        }
                    },
                };

                let player = Player::new(username, uuid);
                let is_admin = editable_settings.admins.contains(&uuid);

                if !player.is_valid() {
                    // Invalid player
                    client.send(ServerRegisterAnswer::Err(RegisterError::InvalidCharacter))?;
                    return Ok(());
                }

                if !players.contains(entity) {
                    // Add Player component to this client
                    let _ = players.insert(entity, player);
                    player_metrics.players_connected.inc();

                    // Give the Admin component to the player if their name exists in
                    // admin list
                    if is_admin {
                        let _ = admins.insert(entity, Admin);
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
        for entity in new_players {
            if let (Some(uid), Some(player)) = (uids.get(entity), players.get(entity)) {
                let mut lazy_msg = None;
                for (_, client) in (&players, &clients).join() {
                    if lazy_msg.is_none() {
                        lazy_msg = Some(client.prepare(ServerGeneral::PlayerListUpdate(
                            PlayerListUpdate::Add(*uid, PlayerInfo {
                                player_alias: player.alias.clone(),
                                is_online: true,
                                is_admin: admins.get(entity).is_some(),
                                character: None, // new players will be on character select.
                            }),
                        )));
                    }
                    lazy_msg.as_ref().map(|ref msg| client.send_prepared(&msg));
                }
            }
        }
    }
}
