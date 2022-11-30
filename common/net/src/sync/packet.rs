use super::track::UpdateTracker;
use common::{resources::Time, uid::Uid};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use specs::{Component, Entity, Join, ReadStorage, World, WorldExt};
use std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
};
use tracing::error;

// TODO: apply_{insert,modify,remove} all take the entity and call
// `write_storage` once per entity per component, instead of once per update
// batch(e.g. in a system-like memory access pattern); if sync ends up being a
// bottleneck, try optimizing this
/// Implemented by type that carries component data for insertion and
/// modification The assocatied `Phantom` type only carries information about
/// which component type is of interest and is used to transmit deletion events
pub trait CompPacket: Clone + Debug + Send + 'static {
    type Phantom: Clone + Debug + Serialize + DeserializeOwned;

    fn apply_insert(self, entity: Entity, world: &World, force_update: bool);
    fn apply_modify(self, entity: Entity, world: &World, force_update: bool);
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
    if let Some(mut c) = world.write_storage::<C>().get_mut(entity) {
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

pub trait InterpolatableComponent: Component {
    type InterpData: Component;
    type ReadData;

    fn new_data(x: Self) -> Self::InterpData;
    fn update_component(&self, data: &mut Self::InterpData, time: f64, force_update: bool);
    #[must_use]
    fn interpolate(self, data: &Self::InterpData, time: f64, read_data: &Self::ReadData) -> Self;
}

pub fn handle_interp_insert<C: InterpolatableComponent + Clone>(
    comp: C,
    entity: Entity,
    world: &World,
    force_update: bool,
) {
    let mut interp_data = C::new_data(comp.clone());
    let time = world.read_resource::<Time>().0;
    comp.update_component(&mut interp_data, time, force_update);
    handle_insert(comp, entity, world);
    handle_insert(interp_data, entity, world);
}

pub fn handle_interp_modify<C: InterpolatableComponent + Debug>(
    comp: C,
    entity: Entity,
    world: &World,
    force_update: bool,
) {
    if let Some(mut interp_data) = world.write_storage::<C::InterpData>().get_mut(entity) {
        let time = world.read_resource::<Time>().0;
        comp.update_component(&mut interp_data, time, force_update);
        handle_modify(comp, entity, world);
    } else {
        error!(
            ?comp,
            "Error modifying interpolation data for synced component, it doesn't seem to exist"
        );
    }
}

pub fn handle_interp_remove<C: InterpolatableComponent>(entity: Entity, world: &World) {
    handle_remove::<C>(entity, world);
    handle_remove::<C::InterpData>(entity, world);
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
pub struct EntitySyncPackage {
    pub created_entities: Vec<u64>,
    pub deleted_entities: Vec<u64>,
}
impl EntitySyncPackage {
    pub fn new(
        uids: &ReadStorage<'_, Uid>,
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
    #[allow(clippy::new_without_default)]
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

    pub fn add_component_updates<'a, C: Component + Clone + Send + Sync>(
        &mut self,
        uids: &ReadStorage<'a, Uid>,
        tracker: &UpdateTracker<C>,
        storage: &ReadStorage<'a, C>,
        filter: impl Join + Copy,
    ) where
        P: From<C>,
        C: TryFrom<P>,
        P::Phantom: From<PhantomData<C>>,
        P::Phantom: TryInto<PhantomData<C>>,
        C::Storage: specs::storage::Tracked,
    {
        tracker.get_updates_for(uids, storage, filter, &mut self.comp_updates);
    }

    /// If there was an update to the component `C` on the provided entity this
    /// will add the update to this package.
    pub fn add_component_update<C: Component + Clone + Send + Sync>(
        &mut self,
        tracker: &UpdateTracker<C>,
        storage: &ReadStorage<'_, C>,
        uid: u64,
        entity: Entity,
    ) where
        P: From<C>,
        C: TryFrom<P>,
        P::Phantom: From<PhantomData<C>>,
        P::Phantom: TryInto<PhantomData<C>>,
        C::Storage: specs::storage::Tracked,
    {
        if let Some(comp_update) = tracker.get_update(storage, entity) {
            self.comp_updates.push((uid, comp_update))
        }
    }

    /// Returns whether this package is empty, useful for not sending an empty
    /// message.
    pub fn is_empty(&self) -> bool { self.comp_updates.is_empty() }
}
