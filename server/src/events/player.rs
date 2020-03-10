use super::Event;
use crate::{auth_provider::AuthProvider, client::Client, state_ext::StateExt, Server};
use common::{
    comp,
    comp::Player,
    msg::{ClientState, PlayerListUpdate, ServerMsg},
    sync::{Uid, UidAllocator},
};
use log::error;
use specs::{saveload::MarkerAllocator, Builder, Entity as EcsEntity, Join, WorldExt};

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
    if let Err(err) = state.delete_entity_recorded(entity) {
        error!("Failed to delete entity when removing character: {:?}", err);
    }
}

pub fn handle_client_disconnect(server: &mut Server, entity: EcsEntity) -> Event {
    let state = server.state_mut();

    // Tell other clients to remove from player list
    if let (Some(uid), Some(_)) = (
        state.read_storage::<Uid>().get(entity),
        state.read_storage::<comp::Player>().get(entity),
    ) {
        state.notify_registered_clients(ServerMsg::PlayerListUpdate(PlayerListUpdate::Remove(
            (*uid).into(),
        )))
    }

    // Make sure to remove the player from the logged in list. (See AuthProvider)
    // And send a disconnected message
    {
        let players = state.ecs().read_storage::<Player>();
        let mut accounts = state.ecs().write_resource::<AuthProvider>();
        let mut clients = state.ecs().write_storage::<Client>();

        if let Some(player) = players.get(entity) {
            accounts.logout(player.uuid());

            let msg = ServerMsg::broadcast(format!("{} went offline.", &player.alias));
            for client in (&mut clients).join().filter(|c| c.is_registered()) {
                client.notify(msg.clone());
            }
        }
    }
    // Delete client entity
    if let Err(err) = state.delete_entity_recorded(entity) {
        error!("Failed to delete disconnected client: {:?}", err);
    }

    Event::ClientDisconnected { entity }
}
