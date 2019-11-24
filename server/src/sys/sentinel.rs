use super::SysTimer;
use common::{
    comp::{
        Body, CanBuild, Gravity, Item, LightEmitter, Mass, MountState, Mounting, Player,
        Projectile, Scale, Stats, Sticky,
    },
    msg::{EcsCompPacket, EcsResPacket},
    state::{Time, TimeOfDay},
    sync::{
        CompPacket, EntityPackage, ResSyncPackage, StatePackage, SyncPackage, Uid, UpdateTracker,
        WorldSyncExt,
    },
};
use shred_derive::SystemData;
use specs::{
    Entity as EcsEntity, Join, ReadExpect, ReadStorage, System, World, Write, WriteExpect,
};
use std::ops::Deref;

/// Always watching
/// This system will monitor specific components for insertion, removal, and modification
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Write<'a, SysTimer<Self>>,
        TrackedComps<'a>,
        WriteTrackers<'a>,
    );

    fn run(&mut self, (mut timer, comps, mut trackers): Self::SystemData) {
        timer.start();

        record_changes(&comps, &mut trackers);

        timer.end();
    }
}

// Probably more difficult than it needs to be :p
#[derive(SystemData)]
pub struct TrackedComps<'a> {
    uid: ReadStorage<'a, Uid>,
    body: ReadStorage<'a, Body>,
    player: ReadStorage<'a, Player>,
    stats: ReadStorage<'a, Stats>,
    can_build: ReadStorage<'a, CanBuild>,
    light_emitter: ReadStorage<'a, LightEmitter>,
    item: ReadStorage<'a, Item>,
    scale: ReadStorage<'a, Scale>,
    mounting: ReadStorage<'a, Mounting>,
    mount_state: ReadStorage<'a, MountState>,
    mass: ReadStorage<'a, Mass>,
    sticky: ReadStorage<'a, Sticky>,
    gravity: ReadStorage<'a, Gravity>,
    projectile: ReadStorage<'a, Projectile>,
}
impl<'a> TrackedComps<'a> {
    pub fn create_entity_package(&self, entity: EcsEntity) -> EntityPackage<EcsCompPacket> {
        let uid = self
            .uid
            .get(entity)
            .copied()
            .expect("No uid to create an entity package")
            .0;
        let mut packets = Vec::new();
        self.body
            .get(entity)
            .copied()
            .map(|c| packets.push(c.into()));
        self.player
            .get(entity)
            .cloned()
            .map(|c| packets.push(c.into()));
        self.stats
            .get(entity)
            .cloned()
            .map(|c| packets.push(c.into()));
        self.can_build
            .get(entity)
            .cloned()
            .map(|c| packets.push(c.into()));
        self.light_emitter
            .get(entity)
            .copied()
            .map(|c| packets.push(c.into()));
        self.item
            .get(entity)
            .cloned()
            .map(|c| packets.push(c.into()));
        self.scale
            .get(entity)
            .copied()
            .map(|c| packets.push(c.into()));
        self.mounting
            .get(entity)
            .cloned()
            .map(|c| packets.push(c.into()));
        self.mount_state
            .get(entity)
            .cloned()
            .map(|c| packets.push(c.into()));
        self.mass
            .get(entity)
            .copied()
            .map(|c| packets.push(c.into()));
        self.sticky
            .get(entity)
            .copied()
            .map(|c| packets.push(c.into()));
        self.gravity
            .get(entity)
            .copied()
            .map(|c| packets.push(c.into()));
        self.projectile
            .get(entity)
            .cloned()
            .map(|c| packets.push(c.into()));

        EntityPackage(uid, packets)
    }
}
#[derive(SystemData)]
pub struct ReadTrackers<'a> {
    uid: ReadExpect<'a, UpdateTracker<Uid>>,
    body: ReadExpect<'a, UpdateTracker<Body>>,
    player: ReadExpect<'a, UpdateTracker<Player>>,
    stats: ReadExpect<'a, UpdateTracker<Stats>>,
    can_build: ReadExpect<'a, UpdateTracker<CanBuild>>,
    light_emitter: ReadExpect<'a, UpdateTracker<LightEmitter>>,
    item: ReadExpect<'a, UpdateTracker<Item>>,
    scale: ReadExpect<'a, UpdateTracker<Scale>>,
    mounting: ReadExpect<'a, UpdateTracker<Mounting>>,
    mount_state: ReadExpect<'a, UpdateTracker<MountState>>,
    mass: ReadExpect<'a, UpdateTracker<Mass>>,
    sticky: ReadExpect<'a, UpdateTracker<Sticky>>,
    gravity: ReadExpect<'a, UpdateTracker<Gravity>>,
    projectile: ReadExpect<'a, UpdateTracker<Projectile>>,
}
impl<'a> ReadTrackers<'a> {
    pub fn create_sync_package(
        &self,
        comps: &TrackedComps,
        filter: impl Join + Copy,
    ) -> SyncPackage<EcsCompPacket> {
        SyncPackage::new(&comps.uid, &self.uid, filter)
            .with_component(
                &comps.uid,
                &self.uid,
                self.body.deref(),
                &comps.body,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.player.deref(),
                &comps.player,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.stats.deref(),
                &comps.stats,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.can_build.deref(),
                &comps.can_build,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.light_emitter.deref(),
                &comps.light_emitter,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.item.deref(),
                &comps.item,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.scale.deref(),
                &comps.scale,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.mounting.deref(),
                &comps.mounting,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.mount_state.deref(),
                &comps.mount_state,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.mass.deref(),
                &comps.mass,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.sticky.deref(),
                &comps.sticky,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.gravity.deref(),
                &comps.gravity,
                filter,
            )
            .with_component(
                &comps.uid,
                &self.uid,
                self.projectile.deref(),
                &comps.projectile,
                filter,
            )
    }
}

#[derive(SystemData)]
pub struct WriteTrackers<'a> {
    uid: WriteExpect<'a, UpdateTracker<Uid>>,
    body: WriteExpect<'a, UpdateTracker<Body>>,
    player: WriteExpect<'a, UpdateTracker<Player>>,
    stats: WriteExpect<'a, UpdateTracker<Stats>>,
    can_build: WriteExpect<'a, UpdateTracker<CanBuild>>,
    light_emitter: WriteExpect<'a, UpdateTracker<LightEmitter>>,
    item: WriteExpect<'a, UpdateTracker<Item>>,
    scale: WriteExpect<'a, UpdateTracker<Scale>>,
    mounting: WriteExpect<'a, UpdateTracker<Mounting>>,
    mount_state: WriteExpect<'a, UpdateTracker<MountState>>,
    mass: WriteExpect<'a, UpdateTracker<Mass>>,
    sticky: WriteExpect<'a, UpdateTracker<Sticky>>,
    gravity: WriteExpect<'a, UpdateTracker<Gravity>>,
    projectile: WriteExpect<'a, UpdateTracker<Projectile>>,
}

fn record_changes(comps: &TrackedComps, trackers: &mut WriteTrackers) {
    // Update trackers
    trackers.uid.record_changes(&comps.uid);
    trackers.body.record_changes(&comps.body);
    trackers.player.record_changes(&comps.player);
    trackers.stats.record_changes(&comps.stats);
    trackers.can_build.record_changes(&comps.can_build);
    trackers.light_emitter.record_changes(&comps.light_emitter);
    trackers.item.record_changes(&comps.item);
    trackers.scale.record_changes(&comps.scale);
    trackers.mounting.record_changes(&comps.mounting);
    trackers.mount_state.record_changes(&comps.mount_state);
    trackers.mass.record_changes(&comps.mass);
    trackers.sticky.record_changes(&comps.sticky);
    trackers.gravity.record_changes(&comps.gravity);
    trackers.projectile.record_changes(&comps.projectile);
}

pub fn register_trackers(world: &mut World) {
    world.register_tracker::<Uid>();
    world.register_tracker::<Body>();
    world.register_tracker::<Player>();
    world.register_tracker::<Stats>();
    world.register_tracker::<CanBuild>();
    world.register_tracker::<LightEmitter>();
    world.register_tracker::<Item>();
    world.register_tracker::<Scale>();
    world.register_tracker::<Mounting>();
    world.register_tracker::<MountState>();
    world.register_tracker::<Mass>();
    world.register_tracker::<Sticky>();
    world.register_tracker::<Gravity>();
    world.register_tracker::<Projectile>();
}

#[derive(SystemData)]
pub struct TrackedResources<'a> {
    time: ReadExpect<'a, Time>,
    time_of_day: ReadExpect<'a, TimeOfDay>,
}
impl<'a> TrackedResources<'a> {
    pub fn create_res_sync_package(&self) -> ResSyncPackage<EcsResPacket> {
        ResSyncPackage::new()
            .with_res(self.time.deref())
            .with_res(self.time_of_day.deref())
    }
    /// Create state package with resources included
    pub fn state_package<C: CompPacket>(&self) -> StatePackage<C, EcsResPacket> {
        StatePackage::new()
            .with_res(self.time.deref())
            .with_res(self.time_of_day.deref())
    }
}
