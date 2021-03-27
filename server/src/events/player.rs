use super::Event;
use crate::{client::Client, persistence, presence::Presence, state_ext::StateExt, Server};
use common::{
    comp,
    comp::group,
    uid::{Uid, UidAllocator},
};
use common_base::span;
use common_net::msg::{PlayerListUpdate, PresenceKind, ServerGeneral};
use common_sys::state::State;
use specs::{saveload::MarkerAllocator, Builder, Entity as EcsEntity, WorldExt};
use tracing::{debug, error, trace, warn, Instrument};

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

    if let Some((client, uid, player)) = (|| {
        let ecs = state.ecs();
        Some((
            ecs.write_storage::<Client>().remove(entity)?,
            ecs.write_storage::<Uid>().remove(entity)?,
            ecs.write_storage::<comp::Player>().remove(entity)?,
        ))
    })() {
        // Tell client its request was successful
        client.send_fallible(ServerGeneral::ExitInGameSuccess);

        let entity_builder = state.ecs_mut().create_entity().with(client).with(player);

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

    // Sync the player's character data to the database
    let entity = persist_entity(state, entity);

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
        server.runtime.spawn(
            async {
                let now = std::time::Instant::now();
                debug!("Start handle disconnect of client");
                if let Err(e) = participant.disconnect().await {
                    debug!(
                        ?e,
                        "Error when disconnecting client, maybe the pipe already broke"
                    );
                };
                trace!("finished disconnect");
                let elapsed = now.elapsed();
                if elapsed.as_millis() > 100 {
                    warn!(?elapsed, "disconnecting took quite long");
                } else {
                    debug!(?elapsed, "disconnecting took");
                }
            }
            .instrument(tracing::debug_span!("client_disconnect", ?pid, ?entity)),
        );
    }

    let state = server.state_mut();

    // Tell other clients to remove from player list
    // And send a disconnected message
    if let (Some(uid), Some(_)) = (
        state.read_storage::<Uid>().get(entity),
        state.read_storage::<comp::Player>().get(entity),
    ) {
        state.notify_players(ServerGeneral::server_msg(comp::ChatType::Offline(*uid), ""));

        state.notify_players(ServerGeneral::PlayerListUpdate(PlayerListUpdate::Remove(
            *uid,
        )));
    }

    // Sync the player's character data to the database
    let entity = persist_entity(state, entity);

    // Delete client entity
    if let Err(e) = state.delete_entity_recorded(entity) {
        error!(?e, ?entity, "Failed to delete disconnected client");
    }

    Event::ClientDisconnected { entity }
}

fn persist_entity(state: &mut State, entity: EcsEntity) -> EcsEntity {
    if let (Some(presences), Some(stats), Some(inventory), updater) = (
        state.read_storage::<Presence>().get(entity),
        state.read_storage::<comp::Stats>().get(entity),
        state.read_storage::<comp::Inventory>().get(entity),
        state
            .ecs()
            .read_resource::<persistence::character_updater::CharacterUpdater>(),
    ) {
        if let PresenceKind::Character(character_id) = presences.kind {
            let waypoint_read = state.read_storage::<comp::Waypoint>();
            let waypoint = waypoint_read.get(entity);
            updater.update(character_id, stats, inventory, waypoint);
        }
    }

    entity
}
