use super::{track::UpdateTracker, uid::Uid};
use log::error;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use specs::{Component, Entity, Join, ReadStorage, World, WorldExt};
use std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
};

/// Implemented by type that carries component data for insertion and modification
/// The assocatied `Phantom` type only carries information about which component type is of
/// interest and is used to transmit deletion events
pub trait CompPacket: Clone + Debug + Send + 'static {
    type Phantom: Clone + Debug + Serialize + DeserializeOwned;

    fn apply_insert(self, entity: Entity, world: &World);
    fn apply_modify(self, entity: Entity, world: &World);
    fn apply_remove(phantom: Self::Phantom, entity: Entity, world: &World);
}

/// Useful for implementing CompPacket trait
pub fn handle_insert<C: Component>(comp: C, entity: Entity, world: &World) {
    if let Err(err) = world.write_storage::<C>().insert(entity, comp) {
        error!("Error inserting component: {:?}", err);
    };
}
/// Useful for implementing CompPacket trait
pub fn handle_modify<C: Component>(comp: C, entity: Entity, world: &World) {
    let _ = world
        .write_storage::<C>()
        .get_mut(entity)
        .map(|c| *c = comp);
}
/// Useful for implementing CompPacket trait
pub fn handle_remove<C: Component>(entity: Entity, world: &World) {
    let _ = world.write_storage::<C>().remove(entity);
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CompUpdateKind<P: CompPacket> {
    Inserted(P),
    Modified(P),
    Removed(P::Phantom),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityPackage<P: CompPacket> {
    pub uid: u64,
    pub comps: Vec<P>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatePackage<P: CompPacket> {
    pub entities: Vec<EntityPackage<P>>,
}

impl<P: CompPacket> Default for StatePackage<P> {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl<P: CompPacket> StatePackage<P> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_entities<C: Component + Clone + Send + Sync>(
        mut self,
        mut entities: Vec<EntityPackage<P>>,
    ) -> Self {
        self.entities.append(&mut entities);
        self
    }
    pub fn with_entity(mut self, entry: EntityPackage<P>) -> Self {
        self.entities.push(entry);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncPackage<P: CompPacket> {
    pub comp_updates: Vec<(u64, CompUpdateKind<P>)>,
    pub created_entities: Vec<u64>,
    pub deleted_entities: Vec<u64>,
}
impl<P: CompPacket> SyncPackage<P> {
    pub fn new<'a>(
        uids: &ReadStorage<'a, Uid>,
        uid_tracker: &UpdateTracker<Uid>,
        filter: impl Join + Copy,
        deleted_entities: Vec<u64>,
    ) -> Self {
        // Add created and deleted entities
        let created_entities = (uids, filter, uid_tracker.inserted())
            .join()
            .map(|(uid, _, _)| (*uid).into())
            .collect();

        Self {
            comp_updates: Vec::new(),
            created_entities,
            deleted_entities,
        }
    }
    pub fn with_component<'a, C: Component + Clone + Send + Sync>(
        mut self,
        uids: &ReadStorage<'a, Uid>,
        tracker: &UpdateTracker<C>,
        storage: &ReadStorage<'a, C>,
        filter: impl Join + Copy,
    ) -> Self
    where
        P: From<C>,
        C: TryFrom<P>,
        P::Phantom: From<PhantomData<C>>,
        P::Phantom: TryInto<PhantomData<C>>,
        C::Storage: specs::storage::Tracked,
    {
        tracker.get_updates_for(uids, storage, filter, &mut self.comp_updates);
        self
    }
}
