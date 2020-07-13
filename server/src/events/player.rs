use super::Event;
use crate::{
    auth_provider::AuthProvider, client::Client, persistence, state_ext::StateExt, Server,
};
use common::{
    comp,
    comp::Player,
    msg::{ClientState, PlayerListUpdate, ServerMsg},
    sync::{Uid, UidAllocator},
};
use futures_executor::block_on;
use specs::{saveload::MarkerAllocator, Builder, Entity as EcsEntity, WorldExt};
use tracing::{debug, error, trace};

pub fn handle_exit_ingame(server: &mut Server, entity: EcsEntity) {
    let state = server.state_mut();

    // Create new entity with just `Client`, `Uid`, and `Player` components
    // Easier than checking and removing all other known components
    // Note: If other `ServerEvent`s are referring to this entity they will be
    // disrupted
    let maybe_client = state.ecs().write_storage::<Client>().remove(entity);
    let maybe_uid = state.read_component_cloned::<Uid>(entity);
    let maybe_player = state.ecs().write_storage::<comp::Player>().remove(entity);
    if let (Some(mut client), Some(uid), Some(player)) = (maybe_client, maybe_uid, maybe_player) {
        // Tell client its request was successful
        client.allow_state(ClientState::Registered);
        // Tell client to clear out other entities and its own components
        client.notify(ServerMsg::ExitIngameCleanup);

        let entity_builder = state.ecs_mut().create_entity().with(client).with(player);
        // Ensure UidAllocator maps this uid to the new entity
        let uid = entity_builder
            .world
            .write_resource::<UidAllocator>()
            .allocate(entity_builder.entity, Some(uid.into()));
        entity_builder.with(uid).build();
    }
    // Delete old entity
    if let Err(e) = state.delete_entity_recorded(entity) {
        error!(
            ?e,
            ?entity,
            "Failed to delete entity when removing character"
        );
    }
}

pub fn handle_client_disconnect(server: &mut Server, entity: EcsEntity) -> Event {
    if let Some(client) = server.state().read_storage::<Client>().get(entity) {
        trace!("Closing participant of client");
        let participant = client.participant.lock().unwrap().take().unwrap();
        if let Err(e) = block_on(participant.disconnect()) {
            debug!(
                ?e,
                "Error when disconnecting client, maybe the pipe already broke"
            );
        };
    }

    let state = server.state_mut();

    // Tell other clients to remove from player list
    if let (Some(uid), Some(_)) = (
        state.read_storage::<Uid>().get(entity),
        state.read_storage::<comp::Player>().get(entity),
    ) {
        state.notify_registered_clients(ServerMsg::PlayerListUpdate(PlayerListUpdate::Remove(*uid)))
    }

    // Make sure to remove the player from the logged in list. (See AuthProvider)
    // And send a disconnected message
    if let Some(player) = state.ecs().read_storage::<Player>().get(entity) {
        let mut accounts = state.ecs().write_resource::<AuthProvider>();
        accounts.logout(player.uuid());

        let msg = comp::ChatType::Offline.server_msg(format!("[{}] went offline.", &player.alias));
        state.notify_registered_clients(msg);
    }

    // Sync the player's character data to the database
    if let (Some(player), Some(stats), Some(inventory), Some(loadout), updater) = (
        state.read_storage::<Player>().get(entity),
        state.read_storage::<comp::Stats>().get(entity),
        state.read_storage::<comp::Inventory>().get(entity),
        state.read_storage::<comp::Loadout>().get(entity),
        state
            .ecs()
            .read_resource::<persistence::character::CharacterUpdater>(),
    ) {
        if let Some(character_id) = player.character_id {
            updater.update(character_id, stats, inventory, loadout);
        }
    }

    // Delete client entity
    if let Err(e) = state.delete_entity_recorded(entity) {
        error!(?e, ?entity, "Failed to delete disconnected client");
    }

    Event::ClientDisconnected { entity }
}
