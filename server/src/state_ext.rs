use crate::{
    client::Client, persistence::PersistedComponents, sys::sentinel::DeletedEntities, SpawnPoint,
};
use common::{
    character::CharacterId,
    comp,
    effect::Effect,
    msg::{
        CharacterInfo, ClientIngame, PlayerListUpdate, ServerGeneralMsg, ServerInGameMsg,
        ServerNotInGameMsg,
    },
    state::State,
    sync::{Uid, UidAllocator, WorldSyncExt},
    util::Dir,
};
use specs::{
    saveload::MarkerAllocator, Builder, Entity as EcsEntity, EntityBuilder as EcsEntityBuilder,
    Join, WorldExt,
};
use tracing::warn;
use vek::*;

pub trait StateExt {
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
    /// Build a shockwave entity
    fn create_shockwave(
        &mut self,
        properties: comp::shockwave::Properties,
        pos: comp::Pos,
        ori: comp::Ori,
    ) -> EcsEntityBuilder;
    /// Build a beam entity
    fn create_beam(
        &mut self,
        properties: comp::beam::Properties,
        pos: comp::Pos,
        ori: comp::Ori,
    ) -> EcsEntityBuilder;
    /// Insert common/default components for a new character joining the server
    fn initialize_character_data(&mut self, entity: EcsEntity, character_id: CharacterId);
    /// Update the components associated with the entity's current character.
    /// Performed after loading component data from the database
    fn update_character_data(&mut self, entity: EcsEntity, components: PersistedComponents);
    /// Iterates over registered clients and send each `ServerMsg`
    fn send_chat(&self, msg: comp::UnresolvedChatMsg);
    fn notify_registered_clients(&self, msg: ServerGeneralMsg);
    /// Delete an entity, recording the deletion in [`DeletedEntities`]
    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration>;
}

impl StateExt for State {
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
                radius: body.radius(),
                z_min: 0.0,
                z_max: body.height(),
            })
            .with(comp::Controller::default())
            .with(body)
            .with(stats)
            .with(comp::Alignment::Npc)
            .with(comp::Energy::new(500))
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
            .with(comp::Mass(5.0))
            .with(comp::Collider::Box {
                radius: comp::Body::Object(object).radius(),
                z_min: 0.0,
                z_max: comp::Body::Object(object).height(),
            })
            .with(comp::Body::Object(object))
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

    fn create_shockwave(
        &mut self,
        properties: comp::shockwave::Properties,
        pos: comp::Pos,
        ori: comp::Ori,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(ori)
            .with(comp::Shockwave {
                properties,
                creation: None,
            })
    }

    fn create_beam(
        &mut self,
        properties: comp::beam::Properties,
        pos: comp::Pos,
        ori: comp::Ori,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(ori)
            .with(comp::BeamSegment {
                properties,
                creation: None,
            })
    }

    fn initialize_character_data(&mut self, entity: EcsEntity, character_id: CharacterId) {
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
        self.write_component(
            entity,
            comp::Alignment::Owned(self.read_component_copied(entity).unwrap()),
        );

        // Make sure physics components are updated
        self.write_component(entity, comp::ForceUpdate);

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

        // Tell the client its request was successful.
        if let Some(client) = self.ecs().write_storage::<Client>().get_mut(entity) {
            client.in_game = Some(ClientIngame::Character);
            client.send_not_in_game(ServerNotInGameMsg::CharacterSuccess)
        }
    }

    fn update_character_data(&mut self, entity: EcsEntity, components: PersistedComponents) {
        let (body, stats, inventory, loadout) = components;

        if let Some(player_uid) = self.read_component_copied::<Uid>(entity) {
            // Notify clients of a player list update
            self.notify_registered_clients(ServerGeneralMsg::PlayerListUpdate(
                PlayerListUpdate::SelectedCharacter(player_uid, CharacterInfo {
                    name: String::from(&stats.name),
                    level: stats.level.level(),
                }),
            ));

            self.write_component(entity, comp::Collider::Box {
                radius: body.radius(),
                z_min: 0.0,
                z_max: body.height(),
            });
            self.write_component(entity, body);
            self.write_component(entity, stats);
            self.write_component(entity, inventory);
            self.write_component(entity, loadout);

            self.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::default()),
            );
        }
    }

    /// Send the chat message to the proper players. Say and region are limited
    /// by location. Faction and group are limited by component.
    fn send_chat(&self, msg: comp::UnresolvedChatMsg) {
        let ecs = self.ecs();
        let is_within =
            |target, a: &comp::Pos, b: &comp::Pos| a.0.distance_squared(b.0) < target * target;

        let group_manager = ecs.read_resource::<comp::group::GroupManager>();
        let resolved_msg = msg.clone().map_group(|group_id| {
            group_manager
                .group_info(group_id)
                .map_or_else(|| "???".into(), |i| i.name.clone())
        });

        match &msg.chat_type {
            comp::ChatType::Online(_)
            | comp::ChatType::Offline(_)
            | comp::ChatType::CommandInfo
            | comp::ChatType::CommandError
            | comp::ChatType::Loot
            | comp::ChatType::Kill(_, _)
            | comp::ChatType::Meta
            | comp::ChatType::World(_) => {
                self.notify_registered_clients(ServerGeneralMsg::ChatMsg(resolved_msg))
            },
            comp::ChatType::Tell(u, t) => {
                for (client, uid) in (
                    &mut ecs.write_storage::<Client>(),
                    &ecs.read_storage::<Uid>(),
                )
                    .join()
                {
                    if uid == u || uid == t {
                        client.send_msg(ServerGeneralMsg::ChatMsg(resolved_msg.clone()));
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
                            client.send_msg(ServerGeneralMsg::ChatMsg(resolved_msg.clone()));
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
                            client.send_msg(ServerGeneralMsg::ChatMsg(resolved_msg.clone()));
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
                            client.send_msg(ServerGeneralMsg::ChatMsg(resolved_msg.clone()));
                        }
                    }
                }
            },

            comp::ChatType::FactionMeta(s) | comp::ChatType::Faction(_, s) => {
                for (client, faction) in (
                    &mut ecs.write_storage::<Client>(),
                    &ecs.read_storage::<comp::Faction>(),
                )
                    .join()
                {
                    if s == &faction.0 {
                        client.send_msg(ServerGeneralMsg::ChatMsg(resolved_msg.clone()));
                    }
                }
            },
            comp::ChatType::GroupMeta(g) | comp::ChatType::Group(_, g) => {
                for (client, group) in (
                    &mut ecs.write_storage::<Client>(),
                    &ecs.read_storage::<comp::Group>(),
                )
                    .join()
                {
                    if g == group {
                        client.send_msg(ServerGeneralMsg::ChatMsg(resolved_msg.clone()));
                    }
                }
            },
        }
    }

    /// Sends the message to all connected clients
    fn notify_registered_clients(&self, msg: ServerGeneralMsg) {
        for client in (&mut self.ecs().write_storage::<Client>())
            .join()
            .filter(|c| c.registered)
        {
            client.send_msg(msg.clone());
        }
    }

    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration> {
        // Remove entity from a group if they are in one
        {
            let mut clients = self.ecs().write_storage::<Client>();
            let uids = self.ecs().read_storage::<Uid>();
            let mut group_manager = self.ecs().write_resource::<comp::group::GroupManager>();
            group_manager.entity_deleted(
                entity,
                &mut self.ecs().write_storage(),
                &self.ecs().read_storage(),
                &uids,
                &self.ecs().entities(),
                &mut |entity, group_change| {
                    clients
                        .get_mut(entity)
                        .and_then(|c| {
                            group_change
                                .try_map(|e| uids.get(e).copied())
                                .map(|g| (g, c))
                        })
                        .map(|(g, c)| c.send_in_game(ServerInGameMsg::GroupUpdate(g)));
                },
            );
        }

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
