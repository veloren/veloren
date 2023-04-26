use super::{
    packet::{CompPacket, CompSyncPackage, CompUpdateKind, EntityPackage, EntitySyncPackage},
    track::UpdateTracker,
};
use common::{
    resources::PlayerEntity,
    uid::{IdMaps, Uid},
};
use specs::{world::Builder, WorldExt};
use tracing::error;

pub trait WorldSyncExt {
    fn register_sync_marker(&mut self);
    fn register_synced<C: specs::Component + Clone + Send + Sync>(&mut self)
    where
        C::Storage: Default + specs::storage::Tracked;
    fn register_tracker<C: specs::Component + Clone + Send + Sync>(&mut self)
    where
        C::Storage: Default + specs::storage::Tracked;
    fn create_entity_synced(&mut self) -> specs::EntityBuilder;
    fn delete_entity_and_clear_from_id_maps(&mut self, uid: Uid);
    fn uid_from_entity(&self, entity: specs::Entity) -> Option<Uid>;
    fn entity_from_uid(&self, uid: Uid) -> Option<specs::Entity>;
    fn apply_entity_package<P: CompPacket>(
        &mut self,
        entity_package: EntityPackage<P>,
    ) -> specs::Entity;
    fn apply_entity_sync_package(&mut self, package: EntitySyncPackage);
    fn apply_comp_sync_package<P: CompPacket>(&mut self, package: CompSyncPackage<P>);
}

impl WorldSyncExt for specs::World {
    fn register_sync_marker(&mut self) {
        self.register_synced::<Uid>();
        self.insert(IdMaps::new());
    }

    fn register_synced<C: specs::Component + Clone + Send + Sync>(&mut self)
    where
        C::Storage: Default + specs::storage::Tracked,
    {
        self.register::<C>();
        self.register_tracker::<C>();
    }

    fn register_tracker<C: specs::Component + Clone + Send + Sync>(&mut self)
    where
        C::Storage: Default + specs::storage::Tracked,
    {
        let tracker = UpdateTracker::<C>::new(self);
        self.insert(tracker);
    }

    fn create_entity_synced(&mut self) -> specs::EntityBuilder {
        // TODO: Add metric for number of new entities created in a tick? Most
        // convenient would be to store counter in `IdMaps` so that we don't
        // have to fetch another resource nor require an additional parameter here
        // nor use globals.
        let builder = self.create_entity();
        let uid = builder
            .world
            .write_resource::<IdMaps>()
            .allocate(builder.entity);
        builder.with(uid)
    }

    /// This method should be used from the client-side when processing network
    /// messages that delete entities.
    // TODO: rename method, document called from client only
    fn delete_entity_and_clear_from_id_maps(&mut self, uid: Uid) {
        // Clear from uid allocator
        let maybe_entity = self
            .write_resource::<IdMaps>()
            .remove_entity(None, uid, None, None);
        if let Some(entity) = maybe_entity {
            if let Err(e) = self.delete_entity(entity) {
                error!(?e, "Failed to delete entity");
            }
        }
    }

    /// Get the UID of an entity
    fn uid_from_entity(&self, entity: specs::Entity) -> Option<Uid> {
        self.read_storage::<Uid>().get(entity).copied()
    }

    /// Get an entity from a UID
    fn entity_from_uid(&self, uid: Uid) -> Option<specs::Entity> {
        self.read_resource::<IdMaps>().uid_entity(uid)
    }

    fn apply_entity_package<P: CompPacket>(
        &mut self,
        entity_package: EntityPackage<P>,
    ) -> specs::Entity {
        let EntityPackage { uid, comps } = entity_package;

        let entity = create_entity_with_uid(self, uid);
        for packet in comps {
            packet.apply_insert(entity, self, true)
        }

        entity
    }

    fn apply_entity_sync_package(&mut self, package: EntitySyncPackage) {
        // Take ownership of the fields
        let EntitySyncPackage {
            created_entities,
            deleted_entities,
        } = package;

        // Attempt to create entities
        created_entities.into_iter().for_each(|uid| {
            create_entity_with_uid(self, uid);
        });

        // Attempt to delete entities that were marked for deletion
        deleted_entities.into_iter().for_each(|uid| {
            self.delete_entity_and_clear_from_id_maps(uid.into());
        });
    }

    fn apply_comp_sync_package<P: CompPacket>(&mut self, package: CompSyncPackage<P>) {
        // Update components
        let player_entity = self.read_resource::<PlayerEntity>().0;
        package.comp_updates.into_iter().for_each(|(uid, update)| {
            if let Some(entity) = self.read_resource::<IdMaps>().uid_entity(uid.into()) {
                let force_update = player_entity == Some(entity);
                match update {
                    CompUpdateKind::Inserted(packet) => {
                        packet.apply_insert(entity, self, force_update);
                    },
                    CompUpdateKind::Modified(packet) => {
                        packet.apply_modify(entity, self, force_update);
                    },
                    CompUpdateKind::Removed(phantom) => {
                        P::apply_remove(phantom, entity, self);
                    },
                }
            }
        });
    }
}

// Private utilities
//
// Only used on the client.
fn create_entity_with_uid(specs_world: &mut specs::World, entity_uid: u64) -> specs::Entity {
    let entity_uid = Uid::from(entity_uid);
    let existing_entity = specs_world.read_resource::<IdMaps>().uid_entity(entity_uid);

    // TODO: Are there any expected cases where there is an existing entity with
    // this UID? If not, we may want to log an error. Otherwise, it may be useful to
    // document these cases.
    match existing_entity {
        Some(entity) => entity,
        None => {
            let entity_builder = specs_world.create_entity();
            entity_builder
                .world
                .write_resource::<IdMaps>()
                .add_entity(entity_uid, entity_builder.entity);
            entity_builder.with(entity_uid).build()
        },
    }
}
