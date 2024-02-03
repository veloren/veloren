use super::Event;
use crate::{
    client::Client, metrics::PlayerMetrics, persistence::character_updater::CharacterUpdater,
    state_ext::StateExt, BattleModeBuffer, Server,
};
use common::{
    character::CharacterId,
    comp,
    comp::{group, pet::is_tameable, Presence, PresenceKind},
    resources::Time,
    uid::{IdMaps, Uid},
};
use common_base::span;
use common_net::msg::{PlayerListUpdate, ServerGeneral};
use common_state::State;
use specs::{Builder, Entity as EcsEntity, Join, WorldExt};
use tracing::{debug, error, trace, warn, Instrument};

pub fn handle_character_delete(
    server: &mut Server,
    entity: EcsEntity,
    requesting_player_uuid: String,
    character_id: CharacterId,
) {
    // Can't process a character delete for a player that has an in-game presence,
    // so kick them out before processing the delete.
    // NOTE: This relies on StateExt::handle_initialize_character adding the
    // Presence component when a character is initialized to detect whether a client
    // is in-game.
    let has_presence = {
        let presences = server.state.ecs().read_storage::<Presence>();
        presences.get(entity).is_some()
    };
    if has_presence {
        warn!(
            ?requesting_player_uuid,
            ?character_id,
            "Character delete received while in-game, disconnecting client."
        );
        handle_exit_ingame(server, entity, true);
    }

    let mut updater = server.state.ecs().fetch_mut::<CharacterUpdater>();
    updater.queue_character_deletion(requesting_player_uuid, character_id);
}

pub fn handle_exit_ingame(server: &mut Server, entity: EcsEntity, skip_persistence: bool) {
    span!(_guard, "handle_exit_ingame");
    let state = server.state_mut();

    // Sync the player's character data to the database. This must be done before
    // removing any components from the entity
    let entity = if !skip_persistence {
        persist_entity(state, entity)
    } else {
        entity
    };

    // Create new entity with just `Client`, `Uid`, `Player`, `Admin`, `Group`
    // components.
    //
    // Easier than checking and removing all other known components.
    //
    // Also, allows clients to not update their Uid based references to this
    // client (e.g. for this specific client's knowledge of its own Uid and for
    // groups since exiting in-game does not affect group membership)
    //
    // Note: If other `ServerEvent`s are referring to this entity they will be
    // disrupted.

    // Cancel trades here since we don't use `delete_entity_recorded` and we
    // remove `Uid` below.
    super::cancel_trades_for(state, entity);

    let maybe_group = state.read_component_copied::<group::Group>(entity);
    let maybe_admin = state.delete_component::<comp::Admin>(entity);
    // Not sure if we still need to actually remove the Uid or if the group
    // logic below relies on this...
    let maybe_uid = state.delete_component::<Uid>(entity);

    if let Some(client) = state.delete_component::<Client>(entity)
        && let Some(uid) = maybe_uid
        && let Some(player) = state.delete_component::<comp::Player>(entity)
    {
        // Tell client its request was successful
        client.send_fallible(ServerGeneral::ExitInGameSuccess);

        let new_entity = state
            .ecs_mut()
            .create_entity()
            .with(client)
            .with(player)
            // Preserve group component if present
            .maybe_with(maybe_group)
            // Preserve admin component if present
            .maybe_with(maybe_admin)
            .with(uid)
            .build();

        // Ensure IdMaps maps this uid to the new entity.
        state.mut_resource::<IdMaps>().remap_entity(uid, new_entity);

        let ecs = state.ecs();
        // Note, we use `delete_entity_common` directly to avoid `delete_entity_recorded` from
        // making any changes to the group.
        if let Some(group) = maybe_group {
            let mut group_manager = ecs.write_resource::<group::GroupManager>();
            if group_manager
                .group_info(group)
                .map(|info| info.leader == entity)
                .unwrap_or(false)
            {
                group_manager.assign_leader(
                    new_entity,
                    &ecs.read_storage(),
                    &ecs.entities(),
                    &ecs.read_storage(),
                    &ecs.read_storage(),
                    // Nothing actually changing since Uid is transferred
                    |_, _| {},
                );
            }
        }
        // delete_entity_recorded` is not used so we don't need to worry aobut
        // group restructuring when deleting this entity.
    } else {
        error!("handle_exit_ingame called with entity that is missing expected components");
    }

    let (maybe_character, sync_me) = state
        .read_storage::<Presence>()
        .get(entity)
        .map(|p| (p.kind.character_id(), p.kind.sync_me()))
        .unzip();
    let maybe_rtsim = state.read_component_copied::<common::rtsim::RtSimEntity>(entity);
    state.mut_resource::<IdMaps>().remove_entity(
        Some(entity),
        None, // Uid re-mapped, we don't want to remove the mapping
        maybe_character.flatten(),
        maybe_rtsim,
    );

    // We don't want to use delete_entity_recorded since we are transfering the
    // Uid to a new entity (and e.g. don't want it to be unmapped).
    //
    // Delete old entity
    if let Err(e) =
        crate::state_ext::delete_entity_common(state, entity, maybe_uid, sync_me.unwrap_or(true))
    {
        error!(
            ?e,
            ?entity,
            "Failed to delete entity when removing character"
        );
    }
}

fn get_reason_str(reason: &comp::DisconnectReason) -> &str {
    match reason {
        comp::DisconnectReason::Timeout => "timeout",
        comp::DisconnectReason::NetworkError => "network_error",
        comp::DisconnectReason::NewerLogin => "newer_login",
        comp::DisconnectReason::Kicked => "kicked",
        comp::DisconnectReason::ClientRequested => "client_requested",
    }
}

pub fn handle_client_disconnect(
    server: &mut Server,
    mut entity: EcsEntity,
    reason: comp::DisconnectReason,
    skip_persistence: bool,
) -> Event {
    span!(_guard, "handle_client_disconnect");
    if let Some(client) = server
        .state()
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
    {
        // NOTE: There are not and likely will not be a way to safeguard against
        // receiving multiple `ServerEvent::ClientDisconnect` messages in a tick
        // intended for the same player, so the `None` case here is *not* a bug
        // and we should not log it as a potential issue.
        server
            .state()
            .ecs()
            .read_resource::<PlayerMetrics>()
            .clients_disconnected
            .with_label_values(&[get_reason_str(&reason)])
            .inc();

        if let Some(participant) = client.participant.take() {
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
                .instrument(tracing::debug_span!(
                    "client_disconnect",
                    ?pid,
                    ?entity,
                    ?reason,
                )),
            );
        }
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
    if !skip_persistence {
        entity = persist_entity(state, entity);
    }

    // Delete client entity
    if let Err(e) = server.state.delete_entity_recorded(entity) {
        error!(?e, ?entity, "Failed to delete disconnected client");
    }

    Event::ClientDisconnected { entity }
}

// When a player logs out, their data is queued for persistence in the next tick
// of the persistence batch update. The player will be
// temporarily unable to log in during this period to avoid
// the race condition of their login fetching their old data
// and overwriting the data saved here.
fn persist_entity(state: &mut State, entity: EcsEntity) -> EcsEntity {
    if let (
        Some(presence),
        Some(skill_set),
        Some(inventory),
        Some(active_abilities),
        Some(player_uid),
        Some(player_info),
        mut character_updater,
        mut battlemode_buffer,
    ) = (
        state.read_storage::<Presence>().get(entity),
        state.read_storage::<comp::SkillSet>().get(entity),
        state.read_storage::<comp::Inventory>().get(entity),
        state
            .read_storage::<comp::ability::ActiveAbilities>()
            .get(entity),
        state.read_storage::<Uid>().get(entity),
        state.read_storage::<comp::Player>().get(entity),
        state.ecs().fetch_mut::<CharacterUpdater>(),
        state.ecs().fetch_mut::<BattleModeBuffer>(),
    ) {
        match presence.kind {
            PresenceKind::LoadingCharacter(_char_id) => {
                error!(
                    "Unexpected state when persist_entity is called! Some of the components \
                     required above should only be present after a character is loaded!"
                );
            },
            PresenceKind::Character(char_id) => {
                let waypoint = state
                    .ecs()
                    .read_storage::<comp::Waypoint>()
                    .get(entity)
                    .cloned();
                let map_marker = state
                    .ecs()
                    .read_storage::<comp::MapMarker>()
                    .get(entity)
                    .cloned();
                // Store last battle mode change
                if let Some(change) = player_info.last_battlemode_change {
                    let mode = player_info.battle_mode;
                    let save = (mode, change);
                    battlemode_buffer.push(char_id, save);
                }

                // Get player's pets
                let alignments = state.ecs().read_storage::<comp::Alignment>();
                let bodies = state.ecs().read_storage::<comp::Body>();
                let stats = state.ecs().read_storage::<comp::Stats>();
                let pets = state.ecs().read_storage::<comp::Pet>();
                let pets = (&alignments, &bodies, &stats, &pets)
                    .join()
                    .filter_map(|(alignment, body, stats, pet)| match alignment {
                        // Don't try to persist non-tameable pets (likely spawned
                        // using /spawn) since there isn't any code to handle
                        // persisting them
                        common::comp::Alignment::Owned(ref pet_owner)
                            if pet_owner == player_uid && is_tameable(body) =>
                        {
                            Some(((*pet).clone(), *body, stats.clone()))
                        },
                        _ => None,
                    })
                    .collect();

                character_updater.add_pending_logout_update((
                    char_id,
                    skill_set.clone(),
                    inventory.clone(),
                    pets,
                    waypoint,
                    active_abilities.clone(),
                    map_marker,
                ));
            },
            PresenceKind::Spectator => { /* Do nothing, spectators do not need persisting */ },
            PresenceKind::Possessor => { /* Do nothing, possessor's are not persisted */ },
        };
    }

    entity
}

/// FIXME: This code is dangerous and needs to be refactored.  We can't just
/// comment it out, but it needs to be fixed for a variety of reasons.  Get rid
/// of this ASAP!
pub fn handle_possess(server: &mut Server, possessor_uid: Uid, possessee_uid: Uid) {
    use crate::presence::RegionSubscription;
    use common::{
        comp::{inventory::slot::EquipSlot, item, slot::Slot, Inventory},
        region::RegionMap,
    };
    use common_net::sync::WorldSyncExt;

    let state = server.state_mut();
    let mut delete_entity = None;

    if let (Some(possessor), Some(possessee)) = (
        state.ecs().entity_from_uid(possessor_uid),
        state.ecs().entity_from_uid(possessee_uid),
    ) {
        // In this section we check various invariants and can return early if any of
        // them are not met.
        let new_presence = {
            let ecs = state.ecs();
            // Check that entities still exist
            if !possessor.gen().is_alive()
                || !ecs.is_alive(possessor)
                || !possessee.gen().is_alive()
                || !ecs.is_alive(possessee)
            {
                error!(
                    "Error possessing! either the possessor entity or possessee entity no longer \
                     exists"
                );
                return;
            }

            let clients = ecs.read_storage::<Client>();
            let players = ecs.read_storage::<comp::Player>();
            let presences = ecs.read_storage::<comp::Presence>();

            if clients.contains(possessee) || players.contains(possessee) {
                error!("Can't possess other players!");
                return;
            }

            if !clients.contains(possessor) {
                error!("Error posessing, no `Client` component on the possessor!");
                return;
            }

            // Limit possessible entities to those in the client's subscribed regions (so
            // that the entity already exists on the client, this reduces the
            // amount of syncing edge cases to consider).
            let subscriptions = ecs.read_storage::<RegionSubscription>();
            let region_map = ecs.read_resource::<RegionMap>();
            let possessee_in_subscribed_region = subscriptions
                .get(possessor)
                .iter()
                .flat_map(|s| s.regions.iter())
                .filter_map(|key| region_map.get(*key))
                .any(|region| region.entities().contains(possessee.id()));
            if !possessee_in_subscribed_region {
                return;
            }

            if let Some(presence) = presences.get(possessor) {
                delete_entity = match presence.kind {
                    k @ (PresenceKind::LoadingCharacter(_) | PresenceKind::Spectator) => {
                        error!(?k, "Unexpected presence kind for a possessor.");
                        return;
                    },
                    PresenceKind::Possessor => None,
                    // Since we call `persist_entity` below we will want to delete the entity (to
                    // avoid item duplication).
                    PresenceKind::Character(_) => Some(possessor),
                };

                Some(Presence {
                    terrain_view_distance: presence.terrain_view_distance,
                    entity_view_distance: presence.entity_view_distance,
                    // This kind (rather than copying Character presence) prevents persistence
                    // from overwriting original character info with stuff from the new character.
                    kind: PresenceKind::Possessor,
                    lossy_terrain_compression: presence.lossy_terrain_compression,
                })
            } else {
                None
            }

            // No early returns allowed after this.
        };

        // Sync the player's character data to the database. This must be done before
        // moving any components from the entity.
        //
        // NOTE: Below we delete old entity (if PresenceKind::Character) as if logging
        // out. This is to prevent any potential for item duplication (although
        // it would only be possible if the player could repossess their entity,
        // hand off some items, and then crash the server in a particular time
        // window, and only admins should have access to the item with this ability
        // in the first place (though that isn't foolproof)). We could potentially fix
        // this but it would require some tweaks to the CharacterUpdater code
        // (to be able to deque the pending persistence request issued here if
        // repossesing the original character), and it seems prudent to be more
        // conservative with making changes there to support this feature.
        let possessor = persist_entity(state, possessor);
        let ecs = state.ecs();

        let mut clients = ecs.write_storage::<Client>();

        // Transfer client component. Note: we require this component for possession.
        let client = clients
            .remove(possessor)
            .expect("Checked client component was present above!");
        client.send_fallible(ServerGeneral::SetPlayerEntity(possessee_uid));
        // Note: we check that the `possessor` and `possessee` entities exist above, so
        // this should never panic.
        clients
            .insert(possessee, client)
            .expect("Checked entity was alive!");

        // Other components to transfer if they exist.
        fn transfer_component<C: specs::Component>(
            storage: &mut specs::WriteStorage<'_, C>,
            possessor: EcsEntity,
            possessee: EcsEntity,
            transform: impl FnOnce(C) -> C,
        ) {
            if let Some(c) = storage.remove(possessor) {
                // Note: we check that the `possessor` and `possessee` entities exist above, so
                // this should never panic.
                storage
                    .insert(possessee, transform(c))
                    .expect("Checked entity was alive!");
            }
        }

        let mut players = ecs.write_storage::<comp::Player>();
        let mut subscriptions = ecs.write_storage::<RegionSubscription>();
        let mut admins = ecs.write_storage::<comp::Admin>();
        let mut waypoints = ecs.write_storage::<comp::Waypoint>();
        let mut force_updates = ecs.write_storage::<comp::ForceUpdate>();

        transfer_component(&mut players, possessor, possessee, |x| x);
        transfer_component(&mut subscriptions, possessor, possessee, |x| x);
        transfer_component(&mut admins, possessor, possessee, |x| x);
        transfer_component(&mut waypoints, possessor, possessee, |x| x);
        let mut update_counter = 0;
        transfer_component(&mut force_updates, possessor, possessee, |mut x| {
            x.update();
            update_counter = x.counter();
            x
        });

        let mut presences = ecs.write_storage::<Presence>();
        // We leave Presence on the old entity for character IDs to be properly removed
        // from the ID mapping if deleting the previous entity.
        //
        // If the entity is not going to be deleted, we remove it so that the entity
        // doesn't keep an area loaded.
        if delete_entity.is_none() {
            presences.remove(possessor);
        }
        if let Some(p) = new_presence {
            presences
                .insert(possessee, p)
                .expect("Checked entity was alive!");
        }

        // If a player is possessing, add possessee to playerlist as player and remove
        // old player.
        // Fetches from possessee entity here since we have transferred over the
        // `Player` component.
        if let Some(player) = players.get(possessee) {
            use common_net::msg;

            let add_player_msg = ServerGeneral::PlayerListUpdate(PlayerListUpdate::Add(
                possessee_uid,
                msg::server::PlayerInfo {
                    player_alias: player.alias.clone(),
                    is_online: true,
                    is_moderator: admins.contains(possessee),
                    character: ecs.read_storage::<comp::Stats>().get(possessee).map(|s| {
                        msg::CharacterInfo {
                            name: s.name.clone(),
                            // NOTE: hack, read docs for humanoid_gender() for more
                            gender: s.original_body.humanoid_gender(),
                        }
                    }),
                    uuid: player.uuid(),
                },
            ));
            let remove_player_msg =
                ServerGeneral::PlayerListUpdate(PlayerListUpdate::Remove(possessor_uid));

            drop((clients, players)); // need to drop so we can use `notify_players` below
            state.notify_players(remove_player_msg);
            state.notify_players(add_player_msg);
        }
        drop(admins);

        // Put possess item into loadout
        let time = ecs.read_resource::<Time>();
        let mut inventories = ecs.write_storage::<Inventory>();
        let mut inventory = inventories
            .entry(possessee)
            .expect("Nobody has &mut World, so there's no way to delete an entity.")
            .or_insert(Inventory::with_empty());

        let debug_item = comp::Item::new_from_asset_expect("common.items.debug.admin_stick");
        if let item::ItemKind::Tool(_) = &*debug_item.kind() {
            let leftover_items = inventory.swap(
                Slot::Equip(EquipSlot::ActiveMainhand),
                Slot::Equip(EquipSlot::InactiveMainhand),
                *time,
            );
            assert!(
                leftover_items.is_empty(),
                "Swapping active and inactive mainhands never results in leftover items"
            );
            inventory.replace_loadout_item(EquipSlot::ActiveMainhand, Some(debug_item), *time);
        }
        drop(inventories);

        // Remove will of the entity
        ecs.write_storage::<comp::Agent>().remove(possessee);
        // Reset controller of former shell
        if let Some(c) = ecs.write_storage::<comp::Controller>().get_mut(possessor) {
            *c = Default::default();
        }

        // Send client new `SyncFrom::ClientEntity` components and tell it to
        // deletes these on the old entity.
        let clients = ecs.read_storage::<Client>();
        let client = clients
            .get(possessee)
            .expect("We insert this component above and have exclusive access to the world.");
        use crate::sys::sentinel::TrackedStorages;
        use specs::SystemData;
        let tracked_storages = TrackedStorages::fetch(ecs);
        let comp_sync_package = tracked_storages.create_sync_from_client_entity_switch(
            possessor_uid,
            possessee_uid,
            possessee,
        );
        if !comp_sync_package.is_empty() {
            client.send_fallible(ServerGeneral::CompSync(comp_sync_package, update_counter));
        }
    }

    // Outside block above to prevent borrow conflicts (i.e. convenient to let
    // everything drop at the end of the block rather than doing it manually for
    // this). See note on `persist_entity` call above for why we do this.
    if let Some(entity) = delete_entity {
        // Delete old entity
        if let Err(e) = state.delete_entity_recorded(entity) {
            error!(
                ?e,
                ?entity,
                "Failed to delete entity when removing character during possession."
            );
        }
    }
}
