use super::{
    packet::{CompPacket, CompUpdateKind, EntityPackage, StatePackage, SyncPackage},
    track::UpdateTracker,
    uid::{Uid, UidAllocator},
};
use log::error;
use specs::{
    saveload::{MarkedBuilder, MarkerAllocator},
    world::Builder,
    WorldExt,
};

pub trait WorldSyncExt {
    fn register_sync_marker(&mut self);
    fn register_synced<C: specs::Component + Clone + Send + Sync>(&mut self)
    where
        C::Storage: Default + specs::storage::Tracked;
    fn register_tracker<C: specs::Component + Clone + Send + Sync>(&mut self)
    where
        C::Storage: Default + specs::storage::Tracked;
    fn create_entity_synced(&mut self) -> specs::EntityBuilder;
    fn delete_entity_and_clear_from_uid_allocator(&mut self, uid: u64);
    fn uid_from_entity(&self, entity: specs::Entity) -> Option<Uid>;
    fn entity_from_uid(&self, uid: u64) -> Option<specs::Entity>;
    fn apply_entity_package<P: CompPacket>(
        &mut self,
        entity_package: EntityPackage<P>,
    ) -> specs::Entity;
    fn apply_state_package<P: CompPacket>(&mut self, state_package: StatePackage<P>);
    fn apply_sync_package<P: CompPacket>(&mut self, package: SyncPackage<P>);
}

impl WorldSyncExt for specs::World {
    fn register_sync_marker(&mut self) {
        self.register_synced::<Uid>();

        // TODO: Consider only having allocator server side for now
        self.insert(UidAllocator::new());
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
        self.create_entity().marked::<super::Uid>()
    }

    /// Get the UID of an entity
    fn uid_from_entity(&self, entity: specs::Entity) -> Option<Uid> {
        self.read_storage::<Uid>().get(entity).copied()
    }

    /// Get the UID of an entity
    fn entity_from_uid(&self, uid: u64) -> Option<specs::Entity> {
        self.read_resource::<UidAllocator>()
            .retrieve_entity_internal(uid)
    }

    fn apply_entity_package<P: CompPacket>(
        &mut self,
        entity_package: EntityPackage<P>,
    ) -> specs::Entity {
        let EntityPackage { uid, comps } = entity_package;

        let entity = create_entity_with_uid(self, uid);
        for packet in comps {
            packet.apply_insert(entity, self)
        }

        entity
    }

    fn delete_entity_and_clear_from_uid_allocator(&mut self, uid: u64) {
        // Clear from uid allocator
        let maybe_entity = self.write_resource::<UidAllocator>().remove_entity(uid);
        if let Some(entity) = maybe_entity {
            if let Err(err) = self.delete_entity(entity) {
                error!("Failed to delete entity: {:?}", err);
            }
        }
    }

    fn apply_state_package<P: CompPacket>(&mut self, state_package: StatePackage<P>) {
        let StatePackage { entities } = state_package;

        // Apply state package entities
        for entity_package in entities {
            self.apply_entity_package(entity_package);
        }

        // TODO: determine if this is needed
        // Initialize entities
        //self.maintain();
    }

    fn apply_sync_package<P: CompPacket>(&mut self, package: SyncPackage<P>) {
        // Take ownership of the fields
        let SyncPackage {
            comp_updates,
            created_entities,
            deleted_entities,
        } = package;

        // Attempt to create entities
        for entity_uid in created_entities {
            create_entity_with_uid(self, entity_uid);
        }

        // Update components
        for (entity_uid, update) in comp_updates {
            if let Some(entity) = self
                .read_resource::<UidAllocator>()
                .retrieve_entity_internal(entity_uid)
            {
                match update {
                    CompUpdateKind::Inserted(packet) => {
                        packet.apply_insert(entity, self);
                    }
                    CompUpdateKind::Modified(packet) => {
                        packet.apply_modify(entity, self);
                    }
                    CompUpdateKind::Removed(phantom) => {
                        P::apply_remove(phantom, entity, self);
                    }
                }
            }
        }

        // Attempt to delete entities that were marked for deletion
        for entity_uid in deleted_entities {
            self.delete_entity_and_clear_from_uid_allocator(entity_uid);
        }
    }
}

// Private utilities
fn create_entity_with_uid(specs_world: &mut specs::World, entity_uid: u64) -> specs::Entity {
    let existing_entity = specs_world
        .read_resource::<UidAllocator>()
        .retrieve_entity_internal(entity_uid);

    match existing_entity {
        Some(entity) => entity,
        None => {
            let entity_builder = specs_world.create_entity();
            let uid = entity_builder
                .world
                .write_resource::<UidAllocator>()
                .allocate(entity_builder.entity, Some(entity_uid));
            entity_builder.with(uid).build()
        }
    }
}
