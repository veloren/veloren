use common::comp::{Admin, Player};
use common_net::msg::{
    ClientRegister, PlayerInfo, PlayerListUpdate, RegisterError, ServerGeneral,
    ServerRegisterAnswer,
};

#[cfg(feature = "plugins")]
use common_sys::plugin::PluginMgr;
use hashbrown::HashMap;
use plugin_api::Uid;
use specs::{
    shred::{Fetch, FetchMut},
    Entity, World, WorldExt, WriteStorage,
};

use crate::{
    client::Client, login_provider::LoginProvider, metrics::PlayerMetrics, EditableSettings,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_register_msg(
    world: &World,
    player_list: &HashMap<Uid, PlayerInfo>,
    new_players: &mut Vec<Entity>,
    entity: Entity,
    client: &Client,
    player_metrics: &Fetch<'_, PlayerMetrics>,
    login_provider: &mut FetchMut<'_, LoginProvider>,
    admins: &mut WriteStorage<'_, Admin>,
    players: &mut WriteStorage<'_, Player>,
    editable_settings: &Fetch<'_, EditableSettings>,
    msg: ClientRegister,
) -> Result<(), crate::error::Error> {
    #[cfg(feature = "plugins")]
    let plugin_mgr = world.read_resource::<PluginMgr>();
    let (username, uuid) = match login_provider.try_login(
        &msg.token_or_username,
        world,
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
}
