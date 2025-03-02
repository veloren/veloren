use crate::comp::{Pos, Presence, Vel};
use common_base::span;
use hashbrown::{DefaultHashBuilder, HashSet};
use indexmap::IndexMap;
use specs::{BitSet, Entities, Join, LendJoin, ReadStorage, hibitset::BitSetLike};
use vek::*;

pub enum RegionEvent {
    /// Contains the key of the region the entity moved to.
    ///
    /// This can be `None` if the `Pos` component is removed or
    /// `Presence::sync_me` becomes `false`.
    ///
    /// If an entity is deleted this event won't be generated.
    Left(u32, Option<Vec2<i32>>),
    /// Contains the key of the region the entity came from.
    Entered(u32, Option<Vec2<i32>>),
}

/// Region consisting of a bitset of entities within it
#[derive(Default)]
pub struct Region {
    // Use specs bitset for simplicity (and joinability)
    bitset: BitSet,
    // TODO consider SmallVec for these
    // Entities that left or entered this region
    events: Vec<RegionEvent>,
}
impl Region {
    /// Checks if the region contains no entities and no events
    fn removable(&self) -> bool { self.bitset.is_empty() && self.events.is_empty() }

    fn add(&mut self, id: u32, from: Option<Vec2<i32>>) {
        self.bitset.add(id);
        self.events.push(RegionEvent::Entered(id, from));
    }

    fn remove(&mut self, id: u32, to: Option<Vec2<i32>>) {
        self.bitset.remove(id);
        self.events.push(RegionEvent::Left(id, to));
    }

    pub fn events(&self) -> &[RegionEvent] { &self.events }

    pub fn entities(&self) -> &BitSet { &self.bitset }
}

/// How far can an entity roam outside its region before it is switched over to
/// the neighboring one In units of blocks (i.e. world pos)
/// Used to prevent rapid switching of entities between regions
pub const TETHER_LENGTH: u32 = 16;
// TODO: This is very coarse for clients with smaller view distances, consider
// using a per client bitset instead of the region system, or at least opt-into
// this for clients with sufficiently small view distances? This could save
// bandwidth. Having a per client bitset would be useful for more complex
// criteria of when to sync although it could take more CPU to maintain, since
// with the region system each entity goes into a single bitset.
//
/// Bitshift between region and world pos, i.e. log2(REGION_SIZE)
///
/// 16 x 16 chunks (if chunks are 32 x 32 blocks)
const REGION_LOG2: u8 = 9;
/// Region size in blocks
pub const REGION_SIZE: u32 = 1 << REGION_LOG2;

#[derive(Default)]
// TODO compare to sweep and prune approach
/// A region system that tracks where entities are.
///
/// Note, this structure is primarily intended for tracking which entities need
/// to be synchronized to which clients (and as part of that what entities are
/// already synchronized). If an entity is marked to not be synchronized to
/// other clients it may not appear here.
pub struct RegionMap {
    // Tree?
    // Sorted Vec? (binary search lookup)
    // Sort into multiple vecs (say 32) using lower bits of morton code, then binary search via
    // upper bits? <-- sounds very promising to me (might not be super good though?)
    regions: IndexMap<Vec2<i32>, Region, DefaultHashBuilder>,
    /// If an entity isn't here it needs to be added to a region.
    tracked_entities: BitSet,
    /// If an entity is in `tracked_entities` this will contain the key of the
    /// region containing that entity.
    ///
    /// Indexed by entity ID.
    ///
    /// Ideally, this would be the index of the region but then we would need to
    /// update this whenever removing regions
    entity_to_region: Vec<Vec2<i32>>,
    // Re-useable vecs
    // (src, entity, pos)
    entities_to_move: Vec<(usize, u32, Vec3<i32>)>,
    // (region, entity)
    entities_to_remove: Vec<(usize, u32)>,
    // Track the current tick, used to enable not checking everything every tick
    // rate is dependent on the rate the caller calls region_manager.tick()
    tick: u64,
}
impl RegionMap {
    pub fn new() -> Self { Self::default() }

    // TODO maintain within a system?
    // TODO special case large entities
    pub fn tick(
        &mut self,
        pos: ReadStorage<Pos>,
        vel: ReadStorage<Vel>,
        presence: ReadStorage<Presence>,
        entities: Entities,
    ) {
        span!(_guard, "tick", "Region::tick");
        self.tick += 1;
        // Clear events within each region
        self.regions.values_mut().for_each(|region| {
            region.events.clear();
        });

        // Add any untracked entities
        for (pos, id) in (&pos, &entities, presence.maybe(), !&self.tracked_entities)
            .join()
            .filter(|(_, _, presence, _)| presence.is_none_or(|p| p.kind.sync_me()))
            .map(|(pos, e, _, _)| (pos, e.id()))
            .collect::<Vec<_>>()
        {
            // Add entity
            self.tracked_entities.add(id);
            self.add_entity(id, pos.0.map(|e| e as i32), None);
        }

        let mut regions_to_remove = Vec::new();

        self.regions
            .iter()
            .enumerate()
            .for_each(|(i, (&current_region, region_data))| {
                for (maybe_pos, _maybe_vel, maybe_presence, id) in (
                    pos.maybe(),
                    vel.maybe(),
                    presence.maybe(),
                    &region_data.bitset,
                )
                    .join()
                {
                    // Entity should already be removed from region bitset if deleted.
                    debug_assert!(entities.is_alive(entities.entity(id)));

                    let should_sync = maybe_presence.is_none_or(|p| p.kind.sync_me());
                    match maybe_pos {
                        // Switch regions for entities which need switching
                        // TODO don't check every tick (use velocity) (and use id to stagger)
                        // Starting parameters at v = 0 check every 100 ticks
                        // tether_length^2 / vel^2  (with a max of every tick)
                        Some(pos) if should_sync => {
                            let pos = pos.0.map(|e| e as i32);
                            let key = Self::pos_key(pos);
                            // Consider switching
                            // Calculate distance outside border
                            if key != current_region
                                && (Vec2::<i32>::from(pos) - Self::key_pos(current_region))
                                    .map(|e| e.unsigned_abs())
                                    .reduce_max()
                                    > TETHER_LENGTH
                            {
                                // Switch
                                self.entities_to_move.push((i, id, pos));
                            }
                        },
                        // Remove any non-existant entities (or just ones that lost their position
                        // component) TODO: distribute this between ticks
                        None | Some(_) => {
                            // TODO: shouldn't there be a way to extract the bitset of entities with
                            // positions directly from specs? Yes, with `.mask()` on the component
                            // storage.
                            self.entities_to_remove.push((i, id));
                        },
                    }
                }

                // Remove region if it is empty and has no events
                // TODO: distribute this between ticks
                if region_data.removable() {
                    regions_to_remove.push(current_region);
                }
            });

        // Mutate
        // Note entity moving is outside the whole loop so that the same entity is not
        // checked twice (this may be fine though...)
        while let Some((i, id, pos)) = self.entities_to_move.pop() {
            // Remove from old region.
            let (prev_key, region) = self.regions.get_index_mut(i).map(|(k, v)| (*k, v)).unwrap();
            region.remove(id, Some(Self::pos_key(pos)));
            // Add to new region.
            self.add_entity(id, pos, Some(prev_key));
        }
        for (i, id) in self.entities_to_remove.drain(..) {
            self.regions
                .get_index_mut(i)
                .map(|(_, v)| v)
                .unwrap()
                .remove(id, None);
            self.tracked_entities.remove(id);
        }
        for key in regions_to_remove.into_iter() {
            // Check that the region is still removable
            if self.regions.get(&key).unwrap().removable() {
                // Note we have to use key's here since the index can change when others are
                // removed
                self.regions.swap_remove(&key);
            }
        }
    }

    /// Must be called immediately after succesfully deleting an entity from the
    /// ecs (i.e. when deleting the entity did not generate a WrongGeneration
    /// error).
    ///
    /// Returns the region key if this entity was tracked in a region.
    pub fn entity_deleted(&mut self, entity: specs::Entity) -> Option<Vec2<i32>> {
        let id = entity.id();
        let was_present = self.tracked_entities.remove(id);
        if was_present {
            // To catch bugs, replace with dummy key.
            let region_key = core::mem::replace(
                &mut self.entity_to_region[id as usize],
                Vec2::from(i32::MAX),
            );
            self.regions
                .get_mut(&region_key)
                .expect("Region must be present if entity was in `tracked_entities`")
                .remove(id, None);
            Some(region_key)
        } else {
            None
        }
    }

    /// Returns index of the region that the entity is added to.
    fn add_entity(&mut self, id: u32, pos: Vec3<i32>, from: Option<Vec2<i32>>) {
        let key = Self::pos_key(pos);
        // Add to region
        self.regions.entry(key).or_default().add(id, from);

        // Add to or update map from entity to region.
        let id = usize::try_from(id).expect("16 bit usize not supported");
        if self.entity_to_region.len() <= id {
            self.entity_to_region.resize(id + 1, Vec2::from(i32::MAX));
        }
        self.entity_to_region[id] = key;
    }

    fn pos_key<P: Into<Vec2<i32>>>(pos: P) -> Vec2<i32> { pos.into().map(|e| e >> REGION_LOG2) }

    pub fn key_pos(key: Vec2<i32>) -> Vec2<i32> { key.map(|e| e << REGION_LOG2) }

    /// Checks if this entity is located in the `RegionMap`.
    pub fn in_region_map(&self, entity: specs::Entity) -> bool {
        self.tracked_entities.contains(entity.id())
    }

    /// Returns a region given a key.
    pub fn get(&self, key: Vec2<i32>) -> Option<&Region> { self.regions.get(&key) }

    /// Returns an iterator of (Position, Region).
    pub fn iter(&self) -> impl Iterator<Item = (Vec2<i32>, &Region)> {
        self.regions.iter().map(|(key, r)| (*key, r))
    }
}

/// Note vd is in blocks in this case
pub fn region_in_vd(key: Vec2<i32>, pos: Vec3<f32>, vd: f32) -> bool {
    let vd_extended = vd + TETHER_LENGTH as f32 * 2.0f32.sqrt();

    let min_region_pos = RegionMap::key_pos(key).map(|e| e as f32);
    // Should be diff to closest point on the square (which can be in the middle of
    // an edge)
    let diff = (min_region_pos - Vec2::from(pos)).map(|e| {
        if e < 0.0 {
            (e + REGION_SIZE as f32).min(0.0)
        } else {
            e
        }
    });

    diff.magnitude_squared() < vd_extended.powi(2)
}

// Note vd is in blocks in this case
pub fn regions_in_vd(pos: Vec3<f32>, vd: f32) -> HashSet<Vec2<i32>> {
    let mut set = HashSet::new();

    let pos_xy = Vec2::<f32>::from(pos);
    let vd_extended = vd + TETHER_LENGTH as f32 * 2.0f32.sqrt();

    let max = RegionMap::pos_key(pos_xy.map(|e| (e + vd_extended) as i32));
    let min = RegionMap::pos_key(pos_xy.map(|e| (e - vd_extended) as i32));

    for x in min.x..max.x + 1 {
        for y in min.y..max.y + 1 {
            let key = Vec2::new(x, y);

            if region_in_vd(key, pos, vd) {
                set.insert(key);
            }
        }
    }

    set
}
// Iterator designed for use in collision systems
// Iterates through all regions yielding them along with half of their neighbors
// ..................

/*fn interleave_i32_with_zeros(mut x: i32) -> i64 {
    x = (x ^ (x << 16)) & 0x0000ffff0000ffff;
    x = (x ^ (x << 8)) & 0x00ff00ff00ff00ff;
    x = (x ^ (x << 4)) & 0x0f0f0f0f0f0f0f0f;
    x = (x ^ (x << 2)) & 0x3333333333333333;
    x = (x ^ (x << 1)) & 0x5555555555555555;
    x
}

fn morton_code(pos: Vec2<i32>) -> i64 {
    interleave_i32_with_zeros(pos.x) | (interleave_i32_with_zeros(pos.y) << 1)
}*/
