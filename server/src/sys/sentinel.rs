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
    ($($component_name:ident: $component_type:ident,)*) => {
        #[derive(SystemData)]
        pub struct TrackedStorages<'a> {
            pub uid: ReadStorage<'a, Uid>,
            $(pub $component_name: ReadStorage<'a, $component_type>,)*
            pub _ability_map: ReadExpect<'a, AbilityMap>,
            pub _msm: ReadExpect<'a, MaterialStatManifest>,
        }

        impl TrackedStorages<'_> {
            pub fn create_entity_package(
                &self,
                entity: EcsEntity,
                pos: Option<Pos>,
                vel: Option<Vel>,
                ori: Option<Ori>,
            ) -> Option<EntityPackage<EcsCompPacket>> {
                let uid = self.uid.get(entity).copied()?.0;
                let mut comps = Vec::new();
                // NOTE: we could potentially include a bitmap indicating which components are present instead of tagging
                // components with the type in order to save bandwidth
                //
                // if the number of optional components sent is less than 1/8 of the number of component types then
                // then the suggested approach would no longer be favorable
                $(

                    if matches!(
                        <$component_type as NetSync>::SYNC_FROM,
                        SyncFrom::AllEntities,
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

                Some(EntityPackage { uid, comps })
            }
        }

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

            /// Update trackers
            fn record_changes(&mut self, comps: &TrackedStorages) {
                self.uid.record_changes(&comps.uid);
                $(
                    self.$component_name.record_changes(&comps.$component_name);
                )*

                const LOG_COUNTS: bool = false;
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
                        SyncFrom::AllEntities,
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


            /// Create sync package for components that are only synced to the client entity
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

use common_net::synced_components::*;
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
        self.map
            .entry(region_key)
            .or_insert(Vec::new())
            .push(uid.into());
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
