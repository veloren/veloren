use super::{
    packet::{
        CompPacket, CompUpdateKind, EntityPackage, ResPacket, ResSyncPackage, StatePackage,
        SyncPackage,
    },
    track::UpdateTracker,
    uid::{Uid, UidAllocator},
};
use specs::{
    saveload::{MarkedBuilder, MarkerAllocator},
    world::Builder,
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
    fn uid_from_entity(&self, entity: specs::Entity) -> Option<Uid>;
    fn entity_from_uid(&self, uid: u64) -> Option<specs::Entity>;
    fn apply_entity_package<P: CompPacket>(&mut self, entity_package: EntityPackage<P>);
    fn apply_state_package<P: CompPacket, R: ResPacket>(
        &mut self,
        state_package: StatePackage<P, R>,
    );
    fn apply_sync_package<P: CompPacket>(&mut self, package: SyncPackage<P>);
    fn apply_res_sync_package<R: ResPacket>(&mut self, package: ResSyncPackage<R>);
}

impl WorldSyncExt for specs::World {
    fn register_sync_marker(&mut self) {
        self.register_synced::<Uid>();

        self.add_resource(UidAllocator::new());
    }
    fn register_synced<C: specs::Component + Clone + Send + Sync>(&mut self)
    where
        //    P: From<C>,
        //    C: TryFrom<P, Error = InvalidType>,
        //    P::Phantom: From<PhantomData<C>>,
        //    P::Phantom: TryInto<PhantomData<C>>,
        C::Storage: Default + specs::storage::Tracked,
    {
        self.register::<C>();
        self.register_tracker::<C>();
    }
    fn register_tracker<C: specs::Component + Clone + Send + Sync>(&mut self)
    where
        //    P: From<C>,
        //    C: TryFrom<P, Error = InvalidType>,
        //    P::Phantom: From<PhantomData<C>>,
        //    P::Phantom: TryInto<PhantomData<C>>,
        C::Storage: Default + specs::storage::Tracked,
    {
        let tracker = UpdateTracker::<C>::new(self);
        self.add_resource(tracker);
    }

    /*fn insert_synced<C: specs::shred::Resource + Clone + Send + Sync>(&mut self, res: C)
    //where
    //    R: From<C>,
    //    C: TryFrom<R>,
    {
        self.add_resource::<C>(res);

        self.res_trackers.insert(ResUpdateTracker::<C>::new());
    }*/

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

    fn apply_entity_package<P: CompPacket>(&mut self, entity_package: EntityPackage<P>) {
        let EntityPackage(entity_uid, packets) = entity_package;

        let entity = create_entity_with_uid(self, entity_uid);
        for packet in packets {
            packet.apply_insert(entity, self)
        }
    }

    fn apply_state_package<P: CompPacket, R: ResPacket>(
        &mut self,
        state_package: StatePackage<P, R>,
    ) {
        let StatePackage {
            entities,
            resources,
        } = state_package;

        // Apply state package resources
        for res_packet in resources {
            res_packet.apply(self);
        }

        // Apply state package entities
        for entity_package in entities {
            self.apply_entity_package(entity_package);
        }

        // Initialize entities
        //specs_world.maintain();
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
            let entity = self
                .read_resource::<UidAllocator>()
                .retrieve_entity_internal(entity_uid);
            if let Some(entity) = entity {
                let _ = self.delete_entity(entity);
            }
        }
    }
    fn apply_res_sync_package<R: ResPacket>(&mut self, package: ResSyncPackage<R>) {
        // Update resources
        for res_packet in package.resources {
            res_packet.apply(self);
        }
    }
}

// Private utilities
fn create_entity_with_uid(specs_world: &mut specs::World, entity_uid: u64) -> specs::Entity {
    let existing_entity = specs_world
        .read_resource::<UidAllocator>()
        .retrieve_entity_internal(entity_uid);

    existing_entity.unwrap_or_else(|| specs_world.create_entity_synced().build())
}
