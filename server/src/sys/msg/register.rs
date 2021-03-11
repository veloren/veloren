use crate::{
    client::Client, login_provider::LoginProvider, metrics::PlayerMetrics, EditableSettings,
};
use common::{
    comp::{Admin, Player, Stats},
    uid::{Uid, UidAllocator},
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{
    CharacterInfo, ClientRegister, PlayerInfo, PlayerListUpdate, RegisterError, ServerGeneral,
    ServerRegisterAnswer,
};
use hashbrown::HashMap;
use plugin_api::Health;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};

#[cfg(feature = "plugins")]
use common_sys::plugin::PluginMgr;

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
        Read<'a, UidAllocator>,
        Read<'a, PluginMgr>,
        ReadStorage<'a, Stats>,
        WriteExpect<'a, LoginProvider>,
        WriteStorage<'a, Admin>,
        ReadExpect<'a, EditableSettings>,
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
            uid_allocator,
            plugin_mgr,
            stats,
            mut login_provider,
            mut admins,
            editable_settings,
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

        for (entity, client) in (&entities, &clients).join() {
            let _ = super::try_recv_all(client, 0, |client, msg: ClientRegister| {
                let (username, uuid) = match login_provider.try_login(
                    &msg.token_or_username,
                    #[cfg(feature = "plugins")]
                    &entities,
                    #[cfg(feature = "plugins")]
                    &health_comp,
                    #[cfg(feature = "plugins")]
                    &uids,
                    #[cfg(feature = "plugins")]
                    &players,
                    #[cfg(feature = "plugins")]
                    &uid_allocator,
                    #[cfg(feature = "plugins")]
                    &plugin_mgr,
                    &*editable_settings.admins,
                    &*editable_settings.whitelist,
                    &*editable_settings.banlist,
                ) {
                    Err(err) => {
                        client.send(ServerRegisterAnswer::Err(err))?;
                        return Ok(());
                    },
                    Ok((username, uuid)) => (username, uuid),
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
            });
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
