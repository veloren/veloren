use crate::{
    client::Client, persistence, settings::ServerSettings, sys::sentinel::DeletedEntities,
    SpawnPoint,
};
use common::{
    assets,
    comp::{self, item},
    effect::Effect,
    msg::{ClientState, RegisterError, RequestStateError, ServerMsg},
    state::State,
    sync::{Uid, WorldSyncExt},
    util::Dir,
};
use log::warn;
use specs::{Builder, Entity as EcsEntity, EntityBuilder as EcsEntityBuilder, Join, WorldExt};
use vek::*;

pub trait StateExt {
    fn give_item(&mut self, entity: EcsEntity, item: comp::Item) -> bool;
    fn apply_effect(&mut self, entity: EcsEntity, effect: Effect);
    fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        loadout: comp::Loadout,
        body: comp::Body,
    ) -> EcsEntityBuilder;
    fn create_object(&mut self, pos: comp::Pos, object: comp::object::Body) -> EcsEntityBuilder;
    fn create_projectile(
        &mut self,
        pos: comp::Pos,
        vel: comp::Vel,
        body: comp::Body,
        projectile: comp::Projectile,
    ) -> EcsEntityBuilder;
    fn create_player_character(
        &mut self,
        entity: EcsEntity,
        character_id: i32,
        body: comp::Body,
        main: Option<String>,
        server_settings: &ServerSettings,
    );
    fn notify_registered_clients(&self, msg: ServerMsg);
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

    /// Build a non-player character.
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

    /// Build a static object entity
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

    /// Build a projectile
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

    fn create_player_character(
        &mut self,
        entity: EcsEntity,
        character_id: i32,
        body: comp::Body,
        main: Option<String>,
        server_settings: &ServerSettings,
    ) {
        // Grab persisted character data from the db and insert their associated
        // components. If for some reason the data can't be returned (missing
        // data, DB error), kick the client back to the character select screen.
        match persistence::character::load_character_data(
            character_id,
            &server_settings.persistence_db_dir,
        ) {
            Ok(stats) => self.write_component(entity, stats),
            Err(error) => {
                log::warn!(
                    "{}",
                    format!(
                        "Failed to load character data for character_id {}: {}",
                        character_id, error
                    )
                );

                if let Some(client) = self.ecs().write_storage::<Client>().get_mut(entity) {
                    client.error_state(RequestStateError::RegisterDenied(
                        RegisterError::InvalidCharacter,
                    ))
                }
            },
        }

        // Give no item when an invalid specifier is given
        let main = main.and_then(|specifier| assets::load_cloned::<comp::Item>(&specifier).ok());

        let spawn_point = self.ecs().read_resource::<SpawnPoint>().0;

        self.write_component(entity, body);
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
        self.write_component(entity, comp::Inventory::default());
        self.write_component(
            entity,
            comp::InventoryUpdate::new(comp::InventoryUpdateEvent::default()),
        );

        self.write_component(
            entity,
            if let Some(item::ItemKind::Tool(tool)) = main.as_ref().map(|i| &i.kind) {
                let mut abilities = tool.get_abilities();
                let mut ability_drain = abilities.drain(..);
                comp::Loadout {
                    active_item: main.map(|item| comp::ItemConfig {
                        item,
                        ability1: ability_drain.next(),
                        ability2: ability_drain.next(),
                        ability3: ability_drain.next(),
                        block_ability: Some(comp::CharacterAbility::BasicBlock),
                        dodge_ability: Some(comp::CharacterAbility::Roll),
                    }),
                    second_item: None,
                    shoulder: None,
                    chest: Some(assets::load_expect_cloned(
                        "common.items.armor.starter.rugged_chest",
                    )),
                    belt: None,
                    hand: None,
                    pants: Some(assets::load_expect_cloned(
                        "common.items.armor.starter.rugged_pants",
                    )),
                    foot: Some(assets::load_expect_cloned(
                        "common.items.armor.starter.sandals_0",
                    )),
                    back: None,
                    ring: None,
                    neck: None,
                    lantern: Some(assets::load_expect_cloned(
                        "common.items.armor.starter.lantern",
                    )),
                    head: None,
                    tabard: None,
                }
            } else {
                comp::Loadout::default()
            },
        );

        // Set the character id for the player
        // TODO this results in a warning in the console: "Error modifying synced
        // component, it doesn't seem to exist"
        // It appears to be caused by the player not yet existing on the client at this
        // point, despite being able to write the data on the server
        &self
            .ecs()
            .write_storage::<comp::Player>()
            .get_mut(entity)
            .map(|player| {
                player.character_id = Some(character_id);
            });

        // Make sure physics are accepted.
        self.write_component(entity, comp::ForceUpdate);

        // Give the Admin component to the player if their name exists in admin list
        if server_settings.admins.contains(
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
