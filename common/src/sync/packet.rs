use super::{
    track::{Tracker, UpdateTracker},
    uid::Uid,
};
use log::error;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use specs::{shred::Resource, Component, Entity, Join, ReadStorage, World};
use std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
};

pub trait CompPacket: Clone + Debug + Send + 'static {
    type Phantom: Clone + Debug + Serialize + DeserializeOwned;

    fn apply_insert(self, entity: Entity, world: &World);
    fn apply_modify(self, entity: Entity, world: &World);
    fn apply_remove(phantom: Self::Phantom, entity: Entity, world: &World);
}

pub trait ResPacket: Clone + Debug + Send + 'static {
    fn apply(self, world: &World);
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
/// Useful for implementing ResPacket trait
pub fn handle_res_update<R: Resource>(res: R, world: &World) {
    *world.write_resource::<R>() = res;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CompUpdateKind<P: CompPacket> {
    Inserted(P),
    Modified(P),
    Removed(P::Phantom),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityPackage<P: CompPacket>(pub u64, pub Vec<P>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatePackage<P: CompPacket, R: ResPacket> {
    pub entities: Vec<EntityPackage<P>>,
    pub resources: Vec<R>,
}

impl<P: CompPacket, R: ResPacket> Default for StatePackage<P, R> {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            resources: Vec::new(),
        }
    }
}

impl<P: CompPacket, R: ResPacket> StatePackage<P, R> {
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
    pub fn with_res<C: Resource + Clone + Send + Sync>(mut self, res: &C) -> Self
    where
        R: From<C>,
    {
        self.resources.push(R::from(res.clone()));
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
        // TODO: handle modified uid?
        //created_entities.append(&mut (uids, filter, uid_tracker.inserted()).join().map(|(uid, _, _)| uid).collect());
        // let deleted_entities = (uids.maybe(), filter, uid_tracker.removed())
        //    .join()
        // Why doesn't this panic??
        //    .map(|(uid, _, _)| Into::<u64>::into(*uid.unwrap()))
        //    .collect::<Vec<_>>();
        //let len = deleted_entities.len();
        //if len > 0 {
        //    println!("deleted {} in sync message", len);
        // }

        Self {
            comp_updates: Vec::new(),
            created_entities,
            deleted_entities,
        }
    }
    pub fn with_component<'a, C: Component + Clone + Send + Sync>(
        mut self,
        uids: &ReadStorage<'a, Uid>,
        tracker: &impl Tracker<C, P>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResSyncPackage<R: ResPacket> {
    pub resources: Vec<R>,
}
impl<R: ResPacket> ResSyncPackage<R> {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }
    pub fn with_res<C: Resource + Clone + Send + Sync>(mut self, res: &C) -> Self
    where
        R: From<C>,
    {
        self.resources.push(R::from(res.clone()));
        self
    }
}
