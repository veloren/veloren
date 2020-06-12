use crate::{
    client::Client, persistence::character::PersistedComponents, settings::ServerSettings,
    sys::sentinel::DeletedEntities, SpawnPoint,
};
use common::{
    comp,
    effect::Effect,
    msg::{CharacterInfo, ClientState, PlayerListUpdate, ServerMsg},
    state::State,
    sync::{Uid, UidAllocator, WorldSyncExt},
    util::Dir,
};
use tracing::warn;
use specs::{
    saveload::MarkerAllocator, Builder, Entity as EcsEntity, EntityBuilder as EcsEntityBuilder,
    Join, WorldExt,
};
use vek::*;

pub trait StateExt {
    /// Push an item into the provided entity's inventory
    fn give_item(&mut self, entity: EcsEntity, item: comp::Item) -> bool;
    /// Updates a component associated with the entity based on the `Effect`
    fn apply_effect(&mut self, entity: EcsEntity, effect: Effect);
    /// Build a non-player character
    fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        loadout: comp::Loadout,
        body: comp::Body,
    ) -> EcsEntityBuilder;
    /// Build a static object entity
    fn create_object(&mut self, pos: comp::Pos, object: comp::object::Body) -> EcsEntityBuilder;
    /// Build a projectile
    fn create_projectile(
        &mut self,
        pos: comp::Pos,
        vel: comp::Vel,
        body: comp::Body,
        projectile: comp::Projectile,
    ) -> EcsEntityBuilder;
    /// Insert common/default components for a new character joining the server
    fn initialize_character_data(&mut self, entity: EcsEntity, character_id: i32);
    /// Update the components associated with the entity's current character.
    /// Performed after loading component data from the database
    fn update_character_data(&mut self, entity: EcsEntity, components: PersistedComponents);
    /// Iterates over registered clients and send each `ServerMsg`
    fn create_player_character(
        &mut self,
        entity: EcsEntity,
        character_id: i32,
        body: comp::Body,
        server_settings: &ServerSettings,
    );
    fn send_chat(&self, msg: comp::ChatMsg);
    fn notify_registered_clients(&self, msg: ServerMsg);
    /// Delete an entity, recording the deletion in [`DeletedEntities`]
    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration>;
}

impl StateExt for State {
    fn give_item(&mut self, entity: EcsEntity, item: comp::Item) -> bool {
        let success = self
            .ecs()
            .write_storage::<comp::Inventory>()
            .get_mut(entity)
            .map(|inv| inv.push(item).is_none())
            .unwrap_or(false);
        if success {
            self.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Collected),
            );
        }
        success
    }

    fn apply_effect(&mut self, entity: EcsEntity, effect: Effect) {
        match effect {
            Effect::Health(change) => {
                self.ecs()
                    .write_storage::<comp::Stats>()
                    .get_mut(entity)
                    .map(|stats| stats.health.change_by(change));
            },
            Effect::Xp(xp) => {
                self.ecs()
                    .write_storage::<comp::Stats>()
                    .get_mut(entity)
                    .map(|stats| stats.exp.change_by(xp));
            },
        }
    }

    fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        loadout: comp::Loadout,
        body: comp::Body,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori::default())
            .with(comp::Collider::Box {
                radius: 0.4,
                z_min: 0.0,
                z_max: 1.75,
            })
            .with(comp::Controller::default())
            .with(body)
            .with(stats)
            .with(comp::Alignment::Npc)
            .with(comp::Energy::new(500))
            .with(comp::Collider::Box {
                radius: 0.4,
                z_min: 0.0,
                z_max: 1.75,
            })
            .with(comp::Gravity(1.0))
            .with(comp::CharacterState::default())
            .with(loadout)
    }

    fn create_object(&mut self, pos: comp::Pos, object: comp::object::Body) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori::default())
            .with(comp::Body::Object(object))
            .with(comp::Mass(100.0))
            .with(comp::Collider::Box {
                radius: 0.4,
                z_min: 0.0,
                z_max: 0.9,
            })
            .with(comp::Gravity(1.0))
    }

    fn create_projectile(
        &mut self,
        pos: comp::Pos,
        vel: comp::Vel,
        body: comp::Body,
        projectile: comp::Projectile,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(vel)
            .with(comp::Ori(Dir::from_unnormalized(vel.0).unwrap_or_default()))
            .with(comp::Mass(0.0))
            .with(comp::Collider::Point)
            .with(body)
            .with(projectile)
            .with(comp::Sticky)
    }

    fn initialize_character_data(&mut self, entity: EcsEntity, character_id: i32) {
        let spawn_point = self.ecs().read_resource::<SpawnPoint>().0;

        self.write_component(entity, comp::Energy::new(1000));
        self.write_component(entity, comp::Controller::default());
        self.write_component(entity, comp::Pos(spawn_point));
        self.write_component(entity, comp::Vel(Vec3::zero()));
        self.write_component(entity, comp::Ori::default());
        self.write_component(entity, comp::Collider::Box {
            radius: 0.4,
            z_min: 0.0,
            z_max: 1.75,
        });
        self.write_component(entity, comp::Gravity(1.0));
        self.write_component(entity, comp::CharacterState::default());
        self.write_component(entity, comp::Alignment::Owned(entity));

        // Set the character id for the player
        // TODO this results in a warning in the console: "Error modifying synced
        // component, it doesn't seem to exist"
        // It appears to be caused by the player not yet existing on the client at this
        // point, despite being able to write the data on the server
        self.ecs()
            .write_storage::<comp::Player>()
            .get_mut(entity)
            .map(|player| {
                player.character_id = Some(character_id);
            });

        // Give the Admin component to the player if their name exists in admin list
        if self.ecs().fetch::<ServerSettings>().admins.contains(
            &self
                .ecs()
                .read_storage::<comp::Player>()
                .get(entity)
                .expect("Failed to fetch entity.")
                .alias,
        ) {
            self.write_component(entity, comp::Admin);
        }

        // Tell the client its request was successful.
        if let Some(client) = self.ecs().write_storage::<Client>().get_mut(entity) {
            client.allow_state(ClientState::Character);
        }
    }

    fn update_character_data(&mut self, entity: EcsEntity, components: PersistedComponents) {
        let (body, stats, inventory, loadout) = components;

        // Notify clients of a player list update
        let client_uid = self
            .read_component_cloned::<Uid>(entity)
            .map(|u| u.into())
            .expect("Client doesn't have a Uid!!!");

        self.notify_registered_clients(ServerMsg::PlayerListUpdate(
            PlayerListUpdate::SelectedCharacter(client_uid, CharacterInfo {
                name: String::from(&stats.name),
                level: stats.level.level(),
            }),
        ));

        self.write_component(entity, body);
        self.write_component(entity, stats);
        self.write_component(entity, inventory);
        self.write_component(entity, loadout);

        self.write_component(
            entity,
            comp::InventoryUpdate::new(comp::InventoryUpdateEvent::default()),
        );

        // Make sure physics are accepted.
        self.write_component(entity, comp::ForceUpdate);
    }

    /// Send the chat message to the proper players. Say and region are limited
    /// by location. Faction and group are limited by component.
    fn send_chat(&self, msg: comp::ChatMsg) {
        let ecs = self.ecs();
        let is_within =
            |target, a: &comp::Pos, b: &comp::Pos| a.0.distance_squared(b.0) < target * target;
        match &msg.chat_type {
            comp::ChatType::Online
            | comp::ChatType::Offline
            | comp::ChatType::CommandInfo
            | comp::ChatType::CommandError
            | comp::ChatType::Kill
            | comp::ChatType::GroupMeta
            | comp::ChatType::FactionMeta
            | comp::ChatType::World(_) => {
                self.notify_registered_clients(ServerMsg::ChatMsg(msg.clone()))
            },
            comp::ChatType::Tell(u, t) => {
                for (client, uid) in (
                    &mut ecs.write_storage::<Client>(),
                    &ecs.read_storage::<Uid>(),
                )
                    .join()
                {
                    if uid == u || uid == t {
                        client.notify(ServerMsg::ChatMsg(msg.clone()));
                    }
                }
            },

            comp::ChatType::Say(uid) => {
                let entity_opt =
                    (*ecs.read_resource::<UidAllocator>()).retrieve_entity_internal(uid.0);
                let positions = ecs.read_storage::<comp::Pos>();
                if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                    for (client, pos) in (&mut ecs.write_storage::<Client>(), &positions).join() {
                        if is_within(comp::ChatMsg::SAY_DISTANCE, pos, speaker_pos) {
                            client.notify(ServerMsg::ChatMsg(msg.clone()));
                        }
                    }
                }
            },
            comp::ChatType::Region(uid) => {
                let entity_opt =
                    (*ecs.read_resource::<UidAllocator>()).retrieve_entity_internal(uid.0);
                let positions = ecs.read_storage::<comp::Pos>();
                if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                    for (client, pos) in (&mut ecs.write_storage::<Client>(), &positions).join() {
                        if is_within(comp::ChatMsg::REGION_DISTANCE, pos, speaker_pos) {
                            client.notify(ServerMsg::ChatMsg(msg.clone()));
                        }
                    }
                }
            },
            comp::ChatType::Npc(uid, _r) => {
                let entity_opt =
                    (*ecs.read_resource::<UidAllocator>()).retrieve_entity_internal(uid.0);
                let positions = ecs.read_storage::<comp::Pos>();
                if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                    for (client, pos) in (&mut ecs.write_storage::<Client>(), &positions).join() {
                        if is_within(comp::ChatMsg::NPC_DISTANCE, pos, speaker_pos) {
                            client.notify(ServerMsg::ChatMsg(msg.clone()));
                        }
                    }
                }
            },

            comp::ChatType::Faction(_u, s) => {
                for (client, faction) in (
                    &mut ecs.write_storage::<Client>(),
                    &ecs.read_storage::<comp::Faction>(),
                )
                    .join()
                {
                    if s == &faction.0 {
                        client.notify(ServerMsg::ChatMsg(msg.clone()));
                    }
                }
            },
            comp::ChatType::Group(_u, s) => {
                for (client, group) in (
                    &mut ecs.write_storage::<Client>(),
                    &ecs.read_storage::<comp::Group>(),
                )
                    .join()
                {
                    if s == &group.0 {
                        client.notify(ServerMsg::ChatMsg(msg.clone()));
                    }
                }
            },
        }
    }

    /// Sends the message to all connected clients
    fn notify_registered_clients(&self, msg: ServerMsg) {
        for client in (&mut self.ecs().write_storage::<Client>())
            .join()
            .filter(|c| c.is_registered())
        {
            client.notify(msg.clone())
        }
    }

    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration> {
        let (maybe_uid, maybe_pos) = (
            self.ecs().read_storage::<Uid>().get(entity).copied(),
            self.ecs().read_storage::<comp::Pos>().get(entity).copied(),
        );
        let res = self.ecs_mut().delete_entity(entity);
        if res.is_ok() {
            if let (Some(uid), Some(pos)) = (maybe_uid, maybe_pos) {
                if let Some(region_key) = self
                    .ecs()
                    .read_resource::<common::region::RegionMap>()
                    .find_region(entity, pos.0)
                {
                    self.ecs()
                        .write_resource::<DeletedEntities>()
                        .record_deleted_entity(uid, region_key);
                } else {
                    // Don't panic if the entity wasn't found in a region maybe it was just created
                    // and then deleted before the region manager had a chance to assign it a
                    // region
                    warn!(
                        ?uid,
                        ?pos,
                        "Failed to find region containing entity during entity deletion, assuming \
                         it wasn't sent to any clients and so deletion doesn't need to be \
                         recorded for sync purposes"
                    );
                }
            }
        }
        res
    }
}
