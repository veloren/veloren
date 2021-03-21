use specs::{world::WorldExt, Builder, Entity as EcsEntity};
use tracing::error;
use vek::*;

use common::{
    comp::{self, agent::AgentEvent, inventory::slot::EquipSlot, item, slot::Slot, Inventory, Pos},
    consts::MAX_MOUNT_RANGE,
    uid::Uid,
    vol::ReadVol,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};

use crate::{
    client::Client,
    presence::{Presence, RegionSubscription},
    state_ext::StateExt,
    Server,
};

pub fn handle_lantern(server: &mut Server, entity: EcsEntity, enable: bool) {
    let ecs = server.state_mut().ecs();

    let lantern_exists = ecs
        .read_storage::<comp::LightEmitter>()
        .get(entity)
        .map_or(false, |light| light.strength > 0.0);

    if lantern_exists != enable {
        if !enable {
            server
                .state_mut()
                .ecs()
                .write_storage::<comp::LightEmitter>()
                .remove(entity);
        } else {
            let inventory_storage = ecs.read_storage::<Inventory>();
            let lantern_opt = inventory_storage
                .get(entity)
                .and_then(|inventory| inventory.equipped(EquipSlot::Lantern))
                .and_then(|item| {
                    if let comp::item::ItemKind::Lantern(l) = item.kind() {
                        Some(l)
                    } else {
                        None
                    }
                });
            if let Some(lantern) = lantern_opt {
                let _ =
                    ecs.write_storage::<comp::LightEmitter>()
                        .insert(entity, comp::LightEmitter {
                            col: lantern.color(),
                            strength: lantern.strength(),
                            flicker: 0.35,
                            animated: true,
                        });
            }
        }
    }
}

pub fn handle_npc_interaction(server: &mut Server, interactor: EcsEntity, npc_entity: EcsEntity) {
    let state = server.state_mut();
    if let Some(agent) = state
        .ecs()
        .write_storage::<comp::Agent>()
        .get_mut(npc_entity)
    {
        if let Some(interactor_uid) = state.ecs().uid_from_entity(interactor) {
            agent.inbox.push_front(AgentEvent::Talk(interactor_uid));
        }
    }
}

pub fn handle_mount(server: &mut Server, mounter: EcsEntity, mountee: EcsEntity) {
    let state = server.state_mut();

    if state
        .ecs()
        .read_storage::<comp::Mounting>()
        .get(mounter)
        .is_none()
    {
        let not_mounting_yet = matches!(
            state.ecs().read_storage::<comp::MountState>().get(mountee),
            Some(comp::MountState::Unmounted)
        );

        let within_range = within_mounting_range(
            state.ecs().read_storage::<comp::Pos>().get(mounter),
            state.ecs().read_storage::<comp::Pos>().get(mountee),
        );

        if not_mounting_yet && within_range {
            if let (Some(mounter_uid), Some(mountee_uid)) = (
                state.ecs().uid_from_entity(mounter),
                state.ecs().uid_from_entity(mountee),
            ) {
                state.write_component(mountee, comp::MountState::MountedBy(mounter_uid));
                state.write_component(mounter, comp::Mounting(mountee_uid));
            }
        }
    }
}

pub fn handle_unmount(server: &mut Server, mounter: EcsEntity) {
    let state = server.state_mut();
    let mountee_entity = state
        .ecs()
        .write_storage::<comp::Mounting>()
        .get(mounter)
        .and_then(|mountee| state.ecs().entity_from_uid(mountee.0.into()));
    if let Some(mountee_entity) = mountee_entity {
        state
            .ecs()
            .write_storage::<comp::MountState>()
            .get_mut(mountee_entity)
            .map(|mut ms| *ms = comp::MountState::Unmounted);
    }
    state.delete_component::<comp::Mounting>(mounter);
}

#[allow(clippy::nonminimal_bool)] // TODO: Pending review in #587
pub fn handle_possess(server: &Server, possessor_uid: Uid, possesse_uid: Uid) {
    let ecs = &server.state.ecs();
    if let (Some(possessor), Some(possesse)) = (
        ecs.entity_from_uid(possessor_uid.into()),
        ecs.entity_from_uid(possesse_uid.into()),
    ) {
        // Check that entities still exist
        if !(possessor.gen().is_alive() && ecs.is_alive(possessor))
            || !(possesse.gen().is_alive() && ecs.is_alive(possesse))
        {
            error!(
                "Error possessing! either the possessor entity or possesse entity no longer exists"
            );
            return;
        }

        if ecs.read_storage::<Client>().get(possesse).is_some() {
            error!("can't possess other players");
            return;
        }

        match (|| -> Option<Result<(), specs::error::Error>> {
            let mut clients = ecs.write_storage::<Client>();
            let c = clients.remove(possessor)?;
            clients.insert(possesse, c).ok()?;
            let playerlist_messages = if let Some(client) = clients.get(possesse) {
                client.send_fallible(ServerGeneral::SetPlayerEntity(possesse_uid));
                // If a player is posessing non player, add possesse to playerlist as player and
                // remove old player
                if let Some(possessor_player) = ecs.read_storage::<comp::Player>().get(possessor) {
                    let admins = ecs.read_storage::<comp::Admin>();
                    let entity_possession_msg = ServerGeneral::PlayerListUpdate(
                        common_net::msg::server::PlayerListUpdate::Add(
                            possesse_uid,
                            common_net::msg::server::PlayerInfo {
                                player_alias: possessor_player.alias.clone(),
                                is_online: true,
                                is_admin: admins.get(possessor).is_some(),
                                character: ecs.read_storage::<comp::Stats>().get(possesse).map(
                                    |s| common_net::msg::CharacterInfo {
                                        name: s.name.clone(),
                                    },
                                ),
                            },
                        ),
                    );
                    let remove_old_player_msg = ServerGeneral::PlayerListUpdate(
                        common_net::msg::server::PlayerListUpdate::Remove(possessor_uid),
                    );

                    // Send msg to new possesse client now because it is not yet considered a player
                    // and will be missed by notify_players
                    client.send_fallible(entity_possession_msg.clone());
                    client.send_fallible(remove_old_player_msg.clone());
                    Some((remove_old_player_msg, entity_possession_msg))
                } else {
                    None
                }
            } else {
                None
            };
            drop(clients);
            if let Some((remove_player, possess_entity)) = playerlist_messages {
                server.state().notify_players(possess_entity);
                server.state().notify_players(remove_player);
            }
            //optional entities
            let mut players = ecs.write_storage::<comp::Player>();
            let mut presence = ecs.write_storage::<Presence>();
            let mut subscriptions = ecs.write_storage::<RegionSubscription>();
            let mut admins = ecs.write_storage::<comp::Admin>();
            let mut waypoints = ecs.write_storage::<comp::Waypoint>();
            players
                .remove(possessor)
                .map(|p| players.insert(possesse, p).ok()?);
            presence
                .remove(possessor)
                .map(|p| presence.insert(possesse, p).ok()?);
            subscriptions
                .remove(possessor)
                .map(|s| subscriptions.insert(possesse, s).ok()?);
            admins
                .remove(possessor)
                .map(|a| admins.insert(possesse, a).ok()?);
            waypoints
                .remove(possessor)
                .map(|w| waypoints.insert(possesse, w).ok()?);

            Some(Ok(()))
        })() {
            Some(Ok(())) => (),
            Some(Err(e)) => {
                error!(?e, ?possesse, "Error inserting component during possession");
                return;
            },
            None => {
                error!(?possessor, "Error removing component during possession");
                return;
            },
        }

        // Put possess item into loadout
        let mut inventories = ecs.write_storage::<Inventory>();
        let mut inventory = inventories
            .entry(possesse)
            .expect("Could not read inventory component while possessing")
            .or_insert(Inventory::new_empty());

        let debug_item = comp::Item::new_from_asset_expect("common.items.debug.admin_stick");
        if let item::ItemKind::Tool(_) = debug_item.kind() {
            inventory
                .swap(
                    Slot::Equip(EquipSlot::Mainhand),
                    Slot::Equip(EquipSlot::Offhand),
                )
                .first()
                .unwrap_none(); // Swapping main and offhand never results in leftover items

            inventory.replace_loadout_item(EquipSlot::Mainhand, Some(debug_item));
        }

        // Remove will of the entity
        ecs.write_storage::<comp::Agent>().remove(possesse);
        // Reset controller of former shell
        ecs.write_storage::<comp::Controller>()
            .get_mut(possessor)
            .map(|c| c.reset());
    }
}

fn within_mounting_range(player_position: Option<&Pos>, mount_position: Option<&Pos>) -> bool {
    match (player_position, mount_position) {
        (Some(ppos), Some(ipos)) => ppos.0.distance_squared(ipos.0) < MAX_MOUNT_RANGE.powi(2),
        _ => false,
    }
}

pub fn handle_mine_block(server: &mut Server, pos: Vec3<i32>) {
    let state = server.state_mut();
    if state.can_set_block(pos) {
        let block = state.terrain().get(pos).ok().copied();
        if let Some(block) = block {
            if let Some(item) = comp::Item::try_reclaim_from_block(block) {
                state
                    .create_object(Default::default(), comp::object::Body::Pouch)
                    .with(comp::Pos(pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)))
                    .with(item)
                    .build();
            }

            state.set_block(pos, block.into_vacant());
        }
    }
}
