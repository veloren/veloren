//! Types of syncing:
//! * synced from all entities
//! * synced only from the client's entity
//!
//! Types of updating
//! * Plain copy of the new component state
//! * (unimplemented) Diff to update component, two variants
//!   * Keep a full copy of the component and generate diff from that
//!   * Intercept changes to the component (no need to compute diff or keep a
//!     full copy)
//!
//! NOTE: rapidly updated components like Pos/Vel/Ori are not covered here

/// Trait that must be implemented for most components that are synced over the
/// network.
pub trait NetSync: specs::Component + Clone + Send + Sync
where
    Self::Storage: specs::storage::Tracked,
{
    // TODO: this scheme theoretically supports diffing withing the
    // impl of `From<UpdateFrom> for Update` but there is no automatic
    // machinery to provide the `UpdateFrom` value yet. Might need to
    // rework this when actuall implementing though.
    //
    //type UpdateFrom = Self;
    //type Update: From<Self::UpdateFrom> = Self;

    const SYNC_FROM: SyncFrom;

    // sync::handle_modify(comp, entity, world)

    /// Allows making modifications before the synced component is inserted on
    /// the client.
    fn pre_insert(&mut self, world: &specs::World) { let _world = world; }

    /// Allows making modifications before the synced component is overwritten
    /// with this version on the client.
    fn pre_modify(&mut self, world: &specs::World) { let _world = world; }
}

/// Whether a component is synced to the client for all entities or for just the
/// client's own entity.
pub enum SyncFrom {
    AllEntities,
    ClientEntity,
}
