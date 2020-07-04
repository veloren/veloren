use super::{track::UpdateTracker, uid::Uid};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use specs::{Component, Entity, Join, ReadStorage, World, WorldExt};
use std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
};
use tracing::error;

/// Implemented by type that carries component data for insertion and
/// modification The assocatied `Phantom` type only carries information about
/// which component type is of interest and is used to transmit deletion events
pub trait CompPacket: Clone + Debug + Send + 'static {
    type Phantom: Clone + Debug + Serialize + DeserializeOwned;

    fn apply_insert(self, entity: Entity, world: &World);
    fn apply_modify(self, entity: Entity, world: &World);
    fn apply_remove(phantom: Self::Phantom, entity: Entity, world: &World);
}

/// Useful for implementing CompPacket trait
pub fn handle_insert<C: Component>(comp: C, entity: Entity, world: &World) {
    if let Err(e) = world.write_storage::<C>().insert(entity, comp) {
        error!(?e, "Error inserting");
    }
}
/// Useful for implementing CompPacket trait
pub fn handle_modify<C: Component + Debug>(comp: C, entity: Entity, world: &World) {
    if let Some(c) = world.write_storage::<C>().get_mut(entity) {
        *c = comp
    } else {
        error!(
            ?comp,
            "Error modifying synced component, it doesn't seem to exist"
        );
    }
}
/// Useful for implementing CompPacket trait
pub fn handle_remove<C: Component>(entity: Entity, world: &World) {
    world.write_storage::<C>().remove(entity);
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
    pub fn new() -> Self { Self::default() }

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
pub struct EntitySyncPackage {
    pub created_entities: Vec<u64>,
    pub deleted_entities: Vec<u64>,
}
impl EntitySyncPackage {
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
            created_entities,
            deleted_entities,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompSyncPackage<P: CompPacket> {
    // TODO: this can be made to take less space by clumping updates for the same entity together
    pub comp_updates: Vec<(u64, CompUpdateKind<P>)>,
}

impl<P: CompPacket> CompSyncPackage<P> {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            comp_updates: Vec::new(),
        }
    }

    pub fn comp_inserted<C>(&mut self, uid: Uid, comp: C)
    where
        P: From<C>,
    {
        self.comp_updates
            .push((uid.into(), CompUpdateKind::Inserted(comp.into())));
    }

    pub fn comp_modified<C>(&mut self, uid: Uid, comp: C)
    where
        P: From<C>,
    {
        self.comp_updates
            .push((uid.into(), CompUpdateKind::Modified(comp.into())));
    }

    pub fn comp_removed<C>(&mut self, uid: Uid)
    where
        P::Phantom: From<PhantomData<C>>,
    {
        self.comp_updates
            .push((uid.into(), CompUpdateKind::Removed(PhantomData::<C>.into())));
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
