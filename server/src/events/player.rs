use super::Event;
use crate::{
    client::Client,
    login_provider::LoginProvider,
    persistence,
    state_ext::StateExt,
    streams::{
        CharacterScreenStream, GeneralStream, GetStream, InGameStream, PingStream, RegisterStream,
    },
    Server,
};
use common::{
    comp,
    comp::{group, Player},
    msg::{PlayerListUpdate, ServerGeneral},
    span,
    sync::{Uid, UidAllocator},
};
use futures_executor::block_on;
use specs::{saveload::MarkerAllocator, Builder, Entity as EcsEntity, WorldExt};
use tracing::{debug, error, trace, warn};

pub fn handle_exit_ingame(server: &mut Server, entity: EcsEntity) {
    span!(_guard, "handle_exit_ingame");
    let state = server.state_mut();

    // Create new entity with just `Client`, `Uid`, `Player`, and `...Stream`
    // components Easier than checking and removing all other known components
    // Note: If other `ServerEvent`s are referring to this entity they will be
    // disrupted

    let maybe_admin = state.ecs().write_storage::<comp::Admin>().remove(entity);
    let maybe_group = state
        .ecs()
        .write_storage::<group::Group>()
        .get(entity)
        .cloned();

    if let Some((
        mut client,
        uid,
        player,
        general_stream,
        ping_stream,
        register_stream,
        character_screen_stream,
        mut in_game_stream,
    )) = (|| {
        let ecs = state.ecs();
        Some((
            ecs.write_storage::<Client>().remove(entity)?,
            ecs.write_storage::<Uid>().remove(entity)?,
            ecs.write_storage::<comp::Player>().remove(entity)?,
            ecs.write_storage::<GeneralStream>().remove(entity)?,
            ecs.write_storage::<PingStream>().remove(entity)?,
            ecs.write_storage::<RegisterStream>().remove(entity)?,
            ecs.write_storage::<CharacterScreenStream>()
                .remove(entity)?,
            ecs.write_storage::<InGameStream>().remove(entity)?,
        ))
    })() {
        // Tell client its request was successful
        client.in_game = None;
        in_game_stream.send_fallible(ServerGeneral::ExitInGameSuccess);

        let entity_builder = state
            .ecs_mut()
            .create_entity()
            .with(client)
            .with(player)
            .with(general_stream)
            .with(ping_stream)
            .with(register_stream)
            .with(character_screen_stream)
            .with(in_game_stream);

        // Preserve group component if present
        let entity_builder = match maybe_group {
            Some(group) => entity_builder.with(group),
            None => entity_builder,
        };

        // Preserve admin component if present
        let entity_builder = match maybe_admin {
            Some(admin) => entity_builder.with(admin),
            None => entity_builder,
        };

        // Ensure UidAllocator maps this uid to the new entity
        let uid = entity_builder
            .world
            .write_resource::<UidAllocator>()
            .allocate(entity_builder.entity, Some(uid.into()));
        let new_entity = entity_builder.with(uid).build();
        if let Some(group) = maybe_group {
            let mut group_manager = state.ecs().write_resource::<group::GroupManager>();
            if group_manager
                .group_info(group)
                .map(|info| info.leader == entity)
                .unwrap_or(false)
            {
                group_manager.assign_leader(
                    new_entity,
                    &state.ecs().read_storage(),
                    &state.ecs().entities(),
                    &state.ecs().read_storage(),
                    &state.ecs().read_storage(),
                    // Nothing actually changing since Uid is transferred
                    |_, _| {},
                );
            }
        }
    }
    // Erase group component to avoid group restructure when deleting the entity
    state.ecs().write_storage::<group::Group>().remove(entity);
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
    span!(_guard, "handle_client_disconnect");
    if let Some(client) = server
        .state()
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
    {
        let participant = client.participant.take().unwrap();
        let pid = participant.remote_pid();
        std::thread::spawn(move || {
            let span = tracing::span!(tracing::Level::DEBUG, "client_disconnect", ?pid, ?entity);
            let _enter = span.enter();
            let now = std::time::Instant::now();
            debug!(?pid, ?entity, "Start handle disconnect of client");
            if let Err(e) = block_on(participant.disconnect()) {
                debug!(
                    ?e,
                    ?pid,
                    "Error when disconnecting client, maybe the pipe already broke"
                );
            };
            trace!(?pid, "finished disconnect");
            let elapsed = now.elapsed();
            if elapsed.as_millis() > 100 {
                warn!(?elapsed, ?pid, "disconnecting took quite long");
            } else {
                debug!(?elapsed, ?pid, "disconnecting took");
            }
        });
    }

    let state = server.state_mut();

    // Tell other clients to remove from player list
    // And send a disconnected message
    if let (Some(uid), Some(_)) = (
        state.read_storage::<Uid>().get(entity),
        state.read_storage::<comp::Player>().get(entity),
    ) {
        state.notify_registered_clients(comp::ChatType::Offline(*uid).server_msg(""));

        state.notify_registered_clients(ServerGeneral::PlayerListUpdate(PlayerListUpdate::Remove(
            *uid,
        )));
    }

    // Make sure to remove the player from the logged in list. (See LoginProvider)
    if let Some(player) = state.ecs().read_storage::<Player>().get(entity) {
        let mut login_provider = state.ecs().write_resource::<LoginProvider>();
        login_provider.logout(player.uuid());
    }

    // Sync the player's character data to the database
    if let (Some(player), Some(stats), Some(inventory), Some(loadout), updater) = (
        state.read_storage::<Player>().get(entity),
        state.read_storage::<comp::Stats>().get(entity),
        state.read_storage::<comp::Inventory>().get(entity),
        state.read_storage::<comp::Loadout>().get(entity),
        state
            .ecs()
            .read_resource::<persistence::character_updater::CharacterUpdater>(),
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
