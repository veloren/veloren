#![allow(clippy::large_enum_variant)]
use common::{
    comp::{
        item::{tool::AbilityMap, MaterialStatManifest},
        Ori, Pos, Vel,
    },
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::{
    msg::EcsCompPacket,
    sync::{CompSyncPackage, EntityPackage, EntitySyncPackage, NetSync, SyncFrom, UpdateTracker},
};
use hashbrown::HashMap;
use specs::{
    shred::ResourceId, Entity as EcsEntity, Join, ReadExpect, ReadStorage, SystemData, World,
    WriteExpect,
};
use vek::*;

/// Always watching
/// This system will monitor specific components for insertion, removal, and
/// modification
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (TrackedStorages<'a>, WriteExpect<'a, UpdateTrackers>);

    const NAME: &'static str = "sentinel";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (storages, mut trackers): Self::SystemData) {
        trackers.record_changes(&storages);
    }
}

/// Holds state like modified bitsets, modification event readers
macro_rules! trackers {
    // Every place where we have `$( /* ... */ )*` will be repeated for each synced component.
    ($($component_name:ident: $component_type:ident,)*) => {
        #[derive(SystemData)]
        pub struct TrackedStorages<'a> {
            // Uids are tracked to detect created entities that should be synced over the network.
            // Additionally we need access to the uids when generating packets to send to the clients.
            pub uid: ReadStorage<'a, Uid>,
            $(pub $component_name: ReadStorage<'a, $component_type>,)*
            // TODO: these may be used to duplicate items when we attempt to remove
            // cloning them.
            pub _ability_map: ReadExpect<'a, AbilityMap>,
            pub _msm: ReadExpect<'a, MaterialStatManifest>,
        }

        impl TrackedStorages<'_> {
            /// Create a package containing all the synced components for this entity. This is
            /// used to initialized the entity's representation on the client (e.g. used when a new
            /// entity is within the area synced to the client).
            ///
            /// Note: This is only for components that are synced to the client for all entities.
            pub fn create_entity_package(
                &self,
                entity: EcsEntity,
                pos: Option<Pos>,
                vel: Option<Vel>,
                ori: Option<Ori>,
            ) -> Option<EntityPackage<EcsCompPacket>> {
                let uid = self.uid.get(entity).copied()?;
                Some(self.create_entity_package_with_uid(entity, uid, pos, vel, ori))
            }

            /// See [create_entity_package].
            ///
            /// NOTE: Only if you're certain you know the UID for the entity, and it hasn't
            /// changed!
            pub fn create_entity_package_with_uid(
                &self,
                entity: EcsEntity,
                uid: Uid,
                pos: Option<Pos>,
                vel: Option<Vel>,
                ori: Option<Ori>,
            ) -> EntityPackage<EcsCompPacket> {
                let uid = uid.0;
                let mut comps = Vec::new();
                // NOTE: we could potentially include a bitmap indicating which components are present instead of tagging
                // components with the type in order to save bandwidth
                //
                // if the number of optional components sent is less than 1/8 of the number of component types then
                // then the suggested approach would no longer be favorable
                $(
                    // Only add components that are synced from any entity.
                    if matches!(
                        <$component_type as NetSync>::SYNC_FROM,
                        SyncFrom::AnyEntity,
                    ) {
                        self
                            .$component_name
                            .get(entity)
                            // TODO: should duplicate rather than clone the item
                            //
                            // NetClone trait?
                            //
                            //.map(|item| item.duplicate(&self.ability_map, &self.msm))
                            .cloned()
                            .map(|c| comps.push(c.into()));
                    }
                )*
                // Add untracked comps
                pos.map(|c| comps.push(c.into()));
                vel.map(|c| comps.push(c.into()));
                ori.map(|c| comps.push(c.into()));

                EntityPackage { uid, comps }
            }

            /// Create sync package for switching a client to another entity specifically to
            /// remove/add components that are only synced for the client's entity.
            pub fn create_sync_from_client_entity_switch(
                &self,
                old_uid: Uid,
                new_uid: Uid,
                new_entity: specs::Entity,
            ) -> CompSyncPackage<EcsCompPacket> {
                let mut comp_sync_package = CompSyncPackage::new();

                $(
                    if matches!(
                        <$component_type as NetSync>::SYNC_FROM,
                        SyncFrom::ClientEntity,
                    ) {
                        comp_sync_package.comp_removed::<$component_type>(old_uid);
                        if let Some(comp) = self.$component_name.get(new_entity).cloned() {
                            comp_sync_package.comp_inserted(new_uid, comp);
                        }
                    }
                )*

                comp_sync_package
            }
        }

        /// Contains an [`UpdateTracker`] for every synced component (that uses this method of
        /// change detection).
        ///
        /// This should be inserted into the ecs as a Resource
        pub struct UpdateTrackers {
            pub uid: UpdateTracker<Uid>,
            $($component_name: UpdateTracker<$component_type>,)*
        }

        impl UpdateTrackers {
            /// Constructs the update trackers and inserts it into the world as a resource.
            ///
            /// Components that will be synced must already be registered.
            pub fn register(world: &mut specs::World) {
                let trackers = UpdateTrackers {
                    uid: UpdateTracker::<Uid>::new(&world),
                    $($component_name: UpdateTracker::<$component_type>::new(&world),)*
                };

                world.insert(trackers);
                // TODO: if we held copies of components for doing diffing, the components that hold that data could be registered here
            }

            /// Records updates to components that are provided from the tracked storages as a series of events into bitsets
            /// that can later be joined on.
            fn record_changes(&mut self, comps: &TrackedStorages) {
                self.uid.record_changes(&comps.uid);
                $(
                    self.$component_name.record_changes(&comps.$component_name);
                )*

                // Enable for logging of counts of component update events.
                const LOG_COUNTS: bool = false;
                // Plotting counts via tracy. Env var provided to toggle on so there's no need to
                // recompile if you are already have a tracy build.
                let plot_counts = common_base::TRACY_ENABLED && matches!(std::env::var("PLOT_UPDATE_COUNTS").as_deref(), Ok("1"));

                macro_rules! log_counts {
                    ($comp:ident, $name:expr) => {
                        if LOG_COUNTS || plot_counts {
                            let tracker = &self.$comp;
                            let inserted = tracker.inserted().into_iter().count();
                            let modified = tracker.modified().into_iter().count();
                            let removed = tracker.removed().into_iter().count();

                            if plot_counts {
                                let sum = inserted + modified + removed;
                                common_base::plot!(concat!($name, "updates"), sum as f64);
                            }

                            if LOG_COUNTS {
                                tracing::warn!("{:6} insertions detected for    {}", inserted, $name);
                                tracing::warn!("{:6} modifications detected for {}", modified, $name);
                                tracing::warn!("{:6} deletions detected for     {}", removed, $name);
                            }
                        }
                    };
                }
                $(log_counts!($component_name, concat!(stringify!($component_name), 's'));)*
            }

            /// Create a [`EntitySyncPackage`] and a [`CompSyncPackage`] to provide updates
            /// for the set entities specified by the provided filter (e.g. for a region).
            ///
            /// A deleted entities must be externally constructed and provided here.
            pub fn create_sync_packages(
                &self,
                comps: &TrackedStorages,
                filter: impl Join + Copy,
                deleted_entities: Vec<u64>,
            ) -> (EntitySyncPackage, CompSyncPackage<EcsCompPacket>) {
                let entity_sync_package =
                    EntitySyncPackage::new(&comps.uid, &self.uid, filter, deleted_entities);
                let mut comp_sync_package = CompSyncPackage::new();

                $(
                    if matches!(
                        <$component_type as NetSync>::SYNC_FROM,
                        SyncFrom::AnyEntity,
                    ) {
                        comp_sync_package.add_component_updates(
                            &comps.uid,
                            &self.$component_name,
                            &comps.$component_name,
                            filter,
                        );
                    }
                )*

                (entity_sync_package, comp_sync_package)
            }


            /// Create sync package for components that are only synced for the client's entity.
            pub fn create_sync_from_client_package(
                &self,
                comps: &TrackedStorages,
                entity: specs::Entity,
            ) -> CompSyncPackage<EcsCompPacket> {
                // TODO: this type repeats the entity uid for each component but
                // we know they will all be the same here, using it for now for
                // convenience but it could help to make a specific type for this
                // later.
                let mut comp_sync_package = CompSyncPackage::new();

                let uid = match comps.uid.get(entity) {
                    Some(uid) => (*uid).into(),
                    // Return empty package if we can't get uid for this entity
                    None => return comp_sync_package,
                };

                $(
                    if matches!(
                        <$component_type as NetSync>::SYNC_FROM,
                        SyncFrom::ClientEntity,
                    ) {
                        comp_sync_package.add_component_update(
                            &self.$component_name,
                            &comps.$component_name,
                            uid,
                            entity,
                        );
                    }
                )*

                comp_sync_package
            }

        }
    }
}

// Import all the component types so they will be available when expanding the
// macro below.
use common_net::synced_components::*;
// Pass `trackers!` macro to this "x macro" which will invoke it with a list
// of components. This will declare the types defined in the macro above.
common_net::synced_components!(trackers);

/// Deleted entities grouped by region
pub struct DeletedEntities {
    map: HashMap<Vec2<i32>, Vec<u64>>,
}

impl Default for DeletedEntities {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl DeletedEntities {
    pub fn record_deleted_entity(&mut self, uid: Uid, region_key: Vec2<i32>) {
        self.map.entry(region_key).or_default().push(uid.into());
    }

    pub fn take_deleted_in_region(&mut self, key: Vec2<i32>) -> Vec<u64> {
        self.map.remove(&key).unwrap_or_default()
    }

    pub fn get_deleted_in_region(&self, key: Vec2<i32>) -> &[u64] {
        self.map.get(&key).map_or(&[], |v| v.as_slice())
    }

    pub fn take_remaining_deleted(&mut self) -> impl Iterator<Item = (Vec2<i32>, Vec<u64>)> + '_ {
        self.map.drain()
    }
}
