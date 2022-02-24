use super::Event;
use crate::{
    client::Client, events::trade::cancel_trade_for, metrics::PlayerMetrics,
    persistence::character_updater::CharacterUpdater, presence::Presence, state_ext::StateExt,
    BattleModeBuffer, Server,
};
use common::{
    comp,
    comp::{group, pet::is_tameable},
    uid::{Uid, UidAllocator},
};
use common_base::span;
use common_net::msg::{PlayerListUpdate, PresenceKind, ServerGeneral};
use common_state::State;
use specs::{saveload::MarkerAllocator, Builder, Entity as EcsEntity, Join, WorldExt};
use tracing::{debug, error, trace, warn, Instrument};

pub fn handle_exit_ingame(server: &mut Server, entity: EcsEntity) {
    span!(_guard, "handle_exit_ingame");
    let state = server.state_mut();

    // Sync the player's character data to the database. This must be done before
    // removing any components from the entity
    let entity = persist_entity(state, entity);

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

    // Delete old entity
    if let Err(e) = state.delete_entity_recorded(entity) {
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
    cancel_trade_for(server, entity);
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
            PresenceKind::Character(char_id) => {
                let waypoint = state
                    .ecs()
                    .read_storage::<common::comp::Waypoint>()
                    .get(entity)
                    .cloned();
                let map_marker = state
                    .ecs()
                    .read_storage::<common::comp::MapMarker>()
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

                character_updater.add_pending_logout_update(
                    char_id,
                    (
                        skill_set.clone(),
                        inventory.clone(),
                        pets,
                        waypoint,
                        active_abilities.clone(),
                        map_marker,
                    ),
                );
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
pub fn handle_possess(server: &mut Server, possessor_uid: Uid, possesse_uid: Uid) {
    use crate::presence::RegionSubscription;
    use common::{
        comp::{inventory::slot::EquipSlot, item, slot::Slot, Inventory},
        region::RegionMap,
    };
    use common_net::sync::WorldSyncExt;

    let state = server.state_mut();
    let mut delete_entity = None;

    if let (Some(possessor), Some(possesse)) = (
        state.ecs().entity_from_uid(possessor_uid.into()),
        state.ecs().entity_from_uid(possesse_uid.into()),
    ) {
        // In this section we check various invariants and can return early if any of
        // them are not met.
        {
            let ecs = state.ecs();
            // Check that entities still exist
            if !possessor.gen().is_alive()
                || !ecs.is_alive(possessor)
                || !possesse.gen().is_alive()
                || !ecs.is_alive(possesse)
            {
                error!(
                    "Error possessing! either the possessor entity or possesse entity no longer \
                     exists"
                );
                return;
            }

            let clients = ecs.read_storage::<Client>();
            let players = ecs.read_storage::<comp::Player>();

            if clients.contains(possesse) || players.contains(possesse) {
                error!("Can't possess other players!");
                return;
            }

            // Limit possessible entities to those in the client's subscribed regions (so
            // that the entity already exists on the client, this reduces the
            // amount of syncing edge cases to consider).
            let subscriptions = ecs.read_storage::<RegionSubscription>();
            let region_map = ecs.read_resource::<RegionMap>();
            let possesse_in_subscribed_region = subscriptions
                .get(possessor)
                .iter()
                .flat_map(|s| s.regions.iter())
                .filter_map(|key| region_map.get(*key))
                .any(|region| region.entities().contains(possesse.id()));
            if !possesse_in_subscribed_region {
                return;
            }

            if !clients.contains(possessor) {
                error!("Error posessing, no `Client` component on the possessor!");
                return;
            }

            // No early returns after this.
        }

        // Sync the player's character data to the database. This must be done before
        // moving any components from the entity.
        //
        // NOTE: Below we delete old entity (if PresenceKind::Character) as if logging out. This is
        // to prevent any potential for item duplication (although it would only be possible if the
        // player could repossess their entity, hand off some items, and then crash the server in a
        // particular time window, and only admins should have access to the item with this ability
        // in the first place (though that isn't foolproof)). We could potentially fix this but it
        // would require some tweaks to the CharacterUpdater code (to be able to deque the pending
        // persistence request issued here if repossesing the original character), and it seems
        // prudent to be more conservative with making changes there to support this feature.
        let possessor = persist_entity(state, possessor);
        let ecs = state.ecs();

        let mut clients = ecs.write_storage::<Client>();

        // Transfer client component. Note: we require this component for possession.
        let client = clients.remove(possessor).expect("Checked client component was present above!");
        client.send_fallible(ServerGeneral::SetPlayerEntity(possesse_uid));
        // Note: we check that the `possessor` and `possesse` entities exist above, so
        // this should never panic.
        clients.insert(possesse, client).expect("Checked entity was alive!");

        // Other components to transfer if they exist.
        fn transfer_component<C: specs::Component>(
            storage: &mut specs::WriteStorage<'_, C>,
            possessor: EcsEntity,
            possesse: EcsEntity,
            transform: impl FnOnce(C) -> C,
        ) {
            if let Some(c) = storage.remove(possessor) {
                // Note: we check that the `possessor` and `possesse` entities exist above, so
                // this should never panic.
                storage
                    .insert(possesse, transform(c))
                    .expect("Checked entity was alive!");
            }
        }

        let mut players = ecs.write_storage::<comp::Player>();
        let mut presence = ecs.write_storage::<Presence>();
        let mut subscriptions = ecs.write_storage::<RegionSubscription>();
        let mut admins = ecs.write_storage::<comp::Admin>();
        let mut waypoints = ecs.write_storage::<comp::Waypoint>();

        transfer_component(&mut players, possessor, possesse, |x| x);
        transfer_component(&mut presence, possessor, possesse, |mut presence| {
            presence.kind = match presence.kind {
                PresenceKind::Spectator => PresenceKind::Spectator,
                // This prevents persistence from overwriting original character info with stuff
                // from the new character.
                PresenceKind::Character(_) => {
                    delete_entity = Some(possessor);
                    PresenceKind::Possessor
                },
                PresenceKind::Possessor => PresenceKind::Possessor,
            };

            presence
        });
        transfer_component(&mut subscriptions, possessor, possesse, |x| x);
        transfer_component(&mut admins, possessor, possesse, |x| x);
        transfer_component(&mut waypoints, possessor, possesse, |x| x);

        // If a player is posessing, add possesse to playerlist as player and remove old
        // player.
        // Fetches from possesse entity here since we have transferred over the `Player`
        // component.
        if let Some(player) = players.get(possesse) {
            use common_net::msg;

            let add_player_msg = ServerGeneral::PlayerListUpdate(
                msg::server::PlayerListUpdate::Add(possesse_uid, msg::server::PlayerInfo {
                    player_alias: player.alias.clone(),
                    is_online: true,
                    is_moderator: admins.contains(possesse),
                    character: ecs.read_storage::<comp::Stats>().get(possesse).map(|s| {
                        msg::CharacterInfo {
                            name: s.name.clone(),
                        }
                    }),
                }),
            );
            let remove_player_msg = ServerGeneral::PlayerListUpdate(
                msg::server::PlayerListUpdate::Remove(possessor_uid),
            );

            drop((clients, players)); // need to drop so we can use `notify_players` below
            state.notify_players(remove_player_msg);
            state.notify_players(add_player_msg);
        }

        // Put possess item into loadout
        let mut inventories = ecs.write_storage::<Inventory>();
        let mut inventory = inventories
            .entry(possesse)
            .expect("Nobody has &mut World, so there's no way to delete an entity.")
            .or_insert(Inventory::new_empty());

        let debug_item = comp::Item::new_from_asset_expect("common.items.debug.admin_stick");
        if let item::ItemKind::Tool(_) = debug_item.kind() {
            let leftover_items = inventory.swap(
                Slot::Equip(EquipSlot::ActiveMainhand),
                Slot::Equip(EquipSlot::InactiveMainhand),
            );
            assert!(
                leftover_items.is_empty(),
                "Swapping active and inactive mainhands never results in leftover items"
            );
            inventory.replace_loadout_item(EquipSlot::ActiveMainhand, Some(debug_item));
        }
        drop(inventories);

        // Remove will of the entity
        ecs.write_storage::<comp::Agent>().remove(possesse);
        // Reset controller of former shell
        if let Some(c) = ecs.write_storage::<comp::Controller>().get_mut(possessor) {
            *c = Default::default();
        }

        // Send client new `SyncFrom::ClientEntity` components and tell it to
        // deletes these on the old entity.
        let clients = ecs.read_storage::<Client>();
        let client = clients
            .get(possesse)
            .expect("We insert this component above and have exclusive access to the world.");
        use crate::sys::sentinel::TrackedStorages;
        use specs::SystemData;
        let tracked_storages = TrackedStorages::fetch(ecs);
        let comp_sync_package = tracked_storages.create_sync_from_client_entity_switch(
            possessor_uid,
            possesse_uid,
            possesse,
        );
        if !comp_sync_package.is_empty() {
            client.send_fallible(ServerGeneral::CompSync(comp_sync_package));
        }
    }

    // Outside block above to prevent borrow conflicts (i.e. convenient to let everything drop at
    // the end of the block rather than doing it manually for this).
    // See note on `persist_entity` call above for why we do this.
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
