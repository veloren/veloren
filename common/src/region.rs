use crate::comp::{Pos, Vel};
use hashbrown::{hash_map::DefaultHashBuilder, HashSet};
use indexmap::IndexMap;
use specs::{hibitset::BitSetLike, BitSet, Entities, Join, ReadStorage};
use vek::*;

pub enum Event {
    // Contains the key of the region the entity moved to
    Left(u32, Option<Vec2<i32>>),
    // Contains the key of the region the entity came from
    Entered(u32, Option<Vec2<i32>>),
}

/// Region consisting of a bitset of entities within it
pub struct Region {
    // Use specs bitset for simplicity (and joinability)
    bitset: BitSet,
    // Indices of neighboring regions
    neighbors: [Option<usize>; 8],
    // TODO consider SmallVec for these
    // Entites that left or entered this region
    events: Vec<Event>,
}
impl Region {
    fn new() -> Self {
        Self {
            bitset: BitSet::new(),
            neighbors: [None; 8],
            events: Vec::new(),
        }
    }

    /// Checks if the region contains no entities and no events
    fn removable(&self) -> bool { self.bitset.is_empty() && self.events.is_empty() }

    fn add(&mut self, id: u32, from: Option<Vec2<i32>>) {
        self.bitset.add(id);
        self.events.push(Event::Entered(id, from));
    }

    fn remove(&mut self, id: u32, to: Option<Vec2<i32>>) {
        self.bitset.remove(id);
        self.events.push(Event::Left(id, to));
    }

    pub fn events(&self) -> &[Event] { &self.events }

    pub fn entities(&self) -> &BitSet { &self.bitset }
}

/// How far can an entity roam outside its region before it is switched over to
/// the neighboring one In units of blocks (i.e. world pos)
/// Used to prevent rapid switching of entities between regions
pub const TETHER_LENGTH: u32 = 16;
/// Bitshift between region and world pos, i.e. log2(REGION_SIZE)
const REGION_LOG2: u8 = 9;
/// Region Size in blocks
pub const REGION_SIZE: u32 = 1 << REGION_LOG2;
/// Offsets to iterate though neighbors
/// Counter-clockwise order
const NEIGHBOR_OFFSETS: [Vec2<i32>; 8] = [
    Vec2::new(0, 1),
    Vec2::new(-1, 1),
    Vec2::new(-1, 0),
    Vec2::new(-1, -1),
    Vec2::new(0, -1),
    Vec2::new(1, -1),
    Vec2::new(1, 0),
    Vec2::new(1, 1),
];

// TODO generic region size (16x16 for now)
// TODO compare to sweep and prune approach
/// A region system that tracks where entities are
pub struct RegionMap {
    // Tree?
    // Sorted Vec? (binary search lookup)
    // Sort into multiple vecs (say 32) using lower bits of morton code, then binary search via
    // upper bits? <-- sounds very promising to me (might not be super good though?)
    regions: IndexMap<Vec2<i32>, Region, DefaultHashBuilder>,
    // If an entity isn't here it needs to be added to a region
    tracked_entities: BitSet,
    // Re-useable vecs
    // (src, entity, pos)
    entities_to_move: Vec<(usize, u32, Vec3<i32>)>,
    // (region, entity)
    entities_to_remove: Vec<(usize, u32)>,
    // Track the current tick, used to enable not checking everything every tick
    tick: u64,
}
impl RegionMap {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            regions: IndexMap::default(),
            tracked_entities: BitSet::new(),
            entities_to_move: Vec::new(),
            entities_to_remove: Vec::new(),
            // rate is depedent on the rate the caller calls region_manager.tick()
            tick: 0,
        }
    }

    // TODO maintain within a system?
    // TODO special case large entities
    pub fn tick(&mut self, pos: ReadStorage<Pos>, vel: ReadStorage<Vel>, entities: Entities) {
        self.tick += 1;
        // Clear events within each region
        for i in 0..self.regions.len() {
            self.regions
                .get_index_mut(i)
                .map(|(_, v)| v)
                .unwrap()
                .events
                .clear();
        }

        // Add any untracked entites
        for (pos, id) in (&pos, &entities, !&self.tracked_entities)
            .join()
            .map(|(pos, e, _)| (pos, e.id()))
            .collect::<Vec<_>>()
        {
            // Add entity
            self.tracked_entities.add(id);
            self.add_entity(id, pos.0.map(|e| e as i32), None);
        }

        let mut regions_to_remove = Vec::new();

        for i in 0..self.regions.len() {
            for (maybe_pos, _maybe_vel, id) in (
                pos.maybe(),
                vel.maybe(),
                &self.regions.get_index(i).map(|(_, v)| v).unwrap().bitset,
            )
                .join()
            {
                match maybe_pos {
                    // Switch regions for entities which need switching
                    // TODO don't check every tick (use velocity) (and use id to stagger)
                    // Starting parameters at v = 0 check every 100 ticks
                    // tether_length^2 / vel^2  (with a max of every tick)
                    Some(pos) => {
                        let pos = pos.0.map(|e| e as i32);
                        let current_region = self.index_key(i).unwrap();
                        let key = Self::pos_key(pos);
                        // Consider switching
                        // Caculate distance outside border
                        if key != current_region
                            && (Vec2::<i32>::from(pos) - Self::key_pos(current_region))
                                .map(|e| e.abs() as u32)
                                .reduce_max()
                                > TETHER_LENGTH
                        {
                            // Switch
                            self.entities_to_move.push((i, id, pos));
                        }
                    },
                    // Remove any non-existant entities (or just ones that lost their position
                    // component) TODO: distribute this between ticks
                    None => {
                        // TODO: shouldn't there be a way to extract the bitset of entities with
                        // positions directly from specs?
                        self.entities_to_remove.push((i, id));
                    },
                }
            }

            // Remove region if it is empty
            // TODO: distribute this betweeen ticks
            let (key, region) = self.regions.get_index(i).unwrap();
            if region.removable() {
                regions_to_remove.push(*key);
            }
        }

        // Mutate
        // Note entity moving is outside the whole loop so that the same entity is not
        // checked twice (this may be fine though...)
        while let Some((i, id, pos)) = self.entities_to_move.pop() {
            let (prev_key, region) = self.regions.get_index_mut(i).map(|(k, v)| (*k, v)).unwrap();
            region.remove(id, Some(Self::pos_key(pos)));

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
                self.remove(key);
            }
        }
    }

    fn add_entity(&mut self, id: u32, pos: Vec3<i32>, from: Option<Vec2<i32>>) {
        let key = Self::pos_key(pos);
        if let Some(region) = self.regions.get_mut(&key) {
            region.add(id, from);
            return;
        }

        let index = self.insert(key);
        self.regions
            .get_index_mut(index)
            .map(|(_, v)| v)
            .unwrap()
            .add(id, None);
    }

    fn pos_key<P: Into<Vec2<i32>>>(pos: P) -> Vec2<i32> { pos.into().map(|e| e >> REGION_LOG2) }

    pub fn key_pos(key: Vec2<i32>) -> Vec2<i32> { key.map(|e| e << REGION_LOG2) }

    /// Finds the region where a given entity is located using a given position
    /// to speed up the search
    #[allow(clippy::needless_range_loop)] // TODO: Pending review in #587
    pub fn find_region(&self, entity: specs::Entity, pos: Vec3<f32>) -> Option<Vec2<i32>> {
        let id = entity.id();
        // Compute key for most likely region
        let key = Self::pos_key(pos.map(|e| e as i32));
        // Get region
        if let Some(region) = self.regions.get(&key) {
            if region.entities().contains(id) {
                return Some(key);
            } else {
                // Check neighbors
                for i in 0..8 {
                    if let Some(idx) = region.neighbors[i] {
                        let (key, region) = self.regions.get_index(idx).unwrap();
                        if region.entities().contains(id) {
                            return Some(*key);
                        }
                    }
                }
            }
        } else {
            // Check neighbors
            for i in 0..8 {
                let key = key + NEIGHBOR_OFFSETS[i];
                if let Some(region) = self.regions.get(&key) {
                    if region.entities().contains(id) {
                        return Some(key);
                    }
                }
            }
        }

        // Scan though all regions
        for (key, region) in self.iter() {
            if region.entities().contains(id) {
                return Some(key);
            }
        }

        None
    }

    fn key_index(&self, key: Vec2<i32>) -> Option<usize> {
        self.regions.get_full(&key).map(|(i, _, _)| i)
    }

    fn index_key(&self, index: usize) -> Option<Vec2<i32>> {
        self.regions.get_index(index).map(|(k, _)| k).copied()
    }

    /// Adds a new region
    /// Returns the index of the region in the index map
    fn insert(&mut self, key: Vec2<i32>) -> usize {
        let (index, old_region) = self.regions.insert_full(key, Region::new());
        if old_region.is_some() {
            panic!("Inserted a region that already exists!!!(this should never need to occur");
        }
        // Add neighbors and add to neighbors
        let mut neighbors = [None; 8];
        for i in 0..8 {
            if let Some((idx, _, region)) = self.regions.get_full_mut(&(key + NEIGHBOR_OFFSETS[i]))
            {
                // Add neighbor to the new region
                neighbors[i] = Some(idx);
                // Add new region to neighbor
                region.neighbors[(i + 4) % 8] = Some(index);
            }
        }
        self.regions
            .get_index_mut(index)
            .map(|(_, v)| v)
            .unwrap()
            .neighbors = neighbors;

        index
    }

    /// Remove a region using its key
    fn remove(&mut self, key: Vec2<i32>) {
        if let Some(index) = self.key_index(key) {
            self.remove_index(index);
        }
    }

    /// Add a region using its key
    #[allow(clippy::needless_range_loop)] // TODO: Pending review in #587
    fn remove_index(&mut self, index: usize) {
        // Remap neighbor indices for neighbors of the region that will be moved from
        // the end of the index map
        if index != self.regions.len() - 1 {
            let moved_neighbors = self
                .regions
                .get_index(self.regions.len() - 1)
                .map(|(_, v)| v)
                .unwrap()
                .neighbors;
            for i in 0..8 {
                if let Some(idx) = moved_neighbors[i] {
                    self.regions
                        .get_index_mut(idx)
                        .map(|(_, v)| v)
                        .unwrap()
                        .neighbors[(i + 4) % 8] = Some(index);
                }
            }
        }
        if let Some(region) = self
            .regions
            .swap_remove_index(index)
            .map(|(_, region)| region)
        {
            if !region.bitset.is_empty() {
                panic!("Removed region containing entities");
            }
            // Remove from neighbors
            for i in 0..8 {
                if let Some(idx) = region.neighbors[i] {
                    self.regions
                        .get_index_mut(idx)
                        .map(|(_, v)| v)
                        .unwrap()
                        .neighbors[(i + 4) % 8] = None;
                }
            }
        }
    }

    // Returns a region given a key
    pub fn get(&self, key: Vec2<i32>) -> Option<&Region> { self.regions.get(&key) }

    // Returns an iterator of (Position, Region)
    pub fn iter(&self) -> impl Iterator<Item = (Vec2<i32>, &Region)> {
        self.regions.iter().map(|(key, r)| (*key, r))
    }
}

// Note vd is in blocks in this case
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
