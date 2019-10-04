use crate::comp::{Pos, Vel};
use hashbrown::hash_map::DefaultHashBuilder;
use hibitset::BitSetLike;
use indexmap::IndexMap;
use specs::{BitSet, Entities, Join, ReadStorage};
use vek::*;

/// Region consisting of a bitset of entities within it
struct Region<S> {
    // Use specs bitset for simplicity (and joinability)
    bitset: BitSet,
    // Indices of neighboring regions
    neighbors: [Option<usize>; 8],
    // Keep track of subscribers
    subscribers: Vec<S>,
}
impl<S> Region<S> {
    fn with_entity(entity: u32) -> Self {
        let mut bitset = BitSet::new();
        bitset.add(entity);
        Self {
            bitset,
            neighbors: [None; 8],
            subscribers: Vec::new(),
        }
    }
}

/// How far can an entity roam outside its region before it is switched over to the neighboring one
/// In units of blocks (i.e. world pos)
/// Used to prevent rapid switching of entities between regions
const TETHER_LENGTH: u32 = 16;
/// Region Size in chunks
const REGION_SIZE: u16 = 16;
const REGION_LOG2: u8 = 4;
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
pub struct RegionMap<S> {
    // Tree?
    // Sorted Vec? (binary search lookup)
    // Sort into multiple vecs (say 32) using lower bits of morton code, then binary search via upper bits? <-- sounds very promising to me (might not be super good though?)
    regions: IndexMap<Vec2<i32>, Region<S>, DefaultHashBuilder>,
    // If an entity isn't here it needs to be added to a region
    tracked_entities: BitSet,
    // Re-useable vecs
    // (src, entity, pos)
    entities_to_move: Vec<(usize, u32, Vec3<i32>)>,
    // (region, entity)
    entities_to_remove: Vec<(usize, u32)>,
}
impl<S> RegionMap<S> {
    pub fn new() -> Self {
        Self {
            regions: IndexMap::default(),
            tracked_entities: BitSet::new(),
            entities_to_move: Vec::new(),
            entities_to_remove: Vec::new(),
        }
    }
    // TODO maintain within a system
    pub fn maintain(
        &mut self,
        pos: ReadStorage<Pos>,
        vel: ReadStorage<Vel>,
        entities: Entities,
        tick: u64,
    ) {
        // Add any untracked entites
        for (pos, id) in (&pos, &entities, !&self.tracked_entities)
            .join()
            .map(|(pos, e, _)| (pos, e.id()))
            .collect::<Vec<_>>()
        {
            // Add entity
            self.add_entity(id, pos.0.map(|e| e as i32));
        }

        self.entities_to_move.clear();
        self.entities_to_remove.clear();

        for i in 0..self.regions.len() {
            for (maybe_pos, maybe_vel, entity) in (
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
                            self.entities_to_move.push((i, entity, pos));
                        }
                    }
                    // Remove any non-existant entities (or just ones that lost their position component)
                    // TODO: distribute this between ticks
                    None => {
                        // TODO: shouldn't there be a way to extract the bitset of entities with positions directly from specs?
                        self.entities_to_remove.push((i, entity));
                    }
                }
            }

            // Remove region if it is empty
            // TODO: distribute this betweeen ticks
            if self
                .regions
                .get_index(i)
                .map(|(_, v)| v)
                .unwrap()
                .bitset
                .is_empty()
            {
                self.remove_index(i);
            }
        }

        // Mutate
        // Note entity moving is outside the whole loop so that the same entity is not checked twice (this may be fine though...)
        while let Some((i, entity, pos)) = self.entities_to_move.pop() {
            self.regions
                .get_index_mut(i)
                .map(|(_, v)| v)
                .unwrap()
                .bitset
                .remove(entity);
            self.add_entity_untracked(entity, pos);
        }
        for (i, entity) in self.entities_to_remove.drain(..) {
            self.regions
                .get_index_mut(i)
                .map(|(_, v)| v)
                .unwrap()
                .bitset
                .remove(entity);
        }

        // Maintain subscriptions ???
    }
    fn add_entity(&mut self, id: u32, pos: Vec3<i32>) {
        self.tracked_entities.add(id);
        self.add_entity_untracked(id, pos);
    }
    fn add_entity_untracked(&mut self, id: u32, pos: Vec3<i32>) {
        let key = Self::pos_key(pos);
        if let Some(region) = self.regions.get_mut(&key) {
            region.bitset.add(id);
            return;
        }

        self.insert(key, id);
    }
    fn pos_key<P: Into<Vec2<i32>>>(pos: P) -> Vec2<i32> {
        pos.into().map(|e| e >> REGION_LOG2)
    }
    fn key_pos(key: Vec2<i32>) -> Vec2<i32> {
        key.map(|e| e << REGION_LOG2)
    }
    fn key_index(&self, key: Vec2<i32>) -> Option<usize> {
        self.regions.get_full(&key).map(|(i, _, _)| i)
    }
    fn index_key(&self, index: usize) -> Option<Vec2<i32>> {
        self.regions.get_index(index).map(|(k, _)| k).copied()
    }
    /// Adds a new region
    fn insert(&mut self, key: Vec2<i32>, entity: u32) {
        let (index, old_region) = self.regions.insert_full(key, Region::with_entity(entity));
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
    }
    /// Remove a region using its key
    fn remove(&mut self, key: Vec2<i32>) {
        if let Some(index) = self.key_index(key) {
            self.remove_index(index);
        }
    }
    /// Add a region using its key
    fn remove_index(&mut self, index: usize) {
        // Remap neighbor indices for neighbors of the region that will be moved from the end of the index map
        let moved_neighbors = self
            .regions
            .get_index(index)
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
}

/*pub struct RegionManager<S> {
    region_map: RegionMap<S>
    // If an entity isn't here it needs to be added to a region
    tracked_entities: BitSet,
}
impl<S> RegionManager {
    // TODO maintain within a system?
    pub fn maintain(&mut self, pos: ReadStorage<Pos>, vel: ReadStorage<Vel>, entities: Entities, tick: u64) {
        let Self {
            ref mut region_map,
            ref mut tracked_entities,
        } =
        // Add any untracked entites
        for (pos, e, _) in (&pos, &entities, !&self.tracked_entities).join() {
            let id = e.id();
            // Add entity
            self.add_entity(id, pos.0.map(|e| e as i32));
        }
        // Iterate through regions
        for i in 0..self.regions.len() {
            for (maybe_pos, maybe_vel, entity) in
                (pos.maybe(), vel.maybe(), &self.regions.get_index(i).map(|(_, v)| v).unwrap().bitset).join()
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
                            self.regions.get_index_mut(i).map(|(_, v)| v).unwrap().bitset.remove(entity);
                            self.add_entity_untracked(entity, pos);
                        }
                    }
                    // Remove any non-existant entities (or just ones that lost their position component)
                    // TODO: distribute this between ticks
                    None => {
                        // TODO: shouldn't there be a way to extract the bitset of entities with positions directly from specs?
                        self.regions.get_index_mut(i).map(|(_, v)| v).unwrap().bitset.remove(entity);
                    }
                }
            }

            // Remove region if it is empty
            // TODO: distribute this betweeen ticks
            if self.regions.get_index(i).map(|(_, v)| v).unwrap().bitset.is_empty() {
                self.remove_index(i);
            }
        }

        // Maintain subscriptions ???

    }
}*/
// Iterator designed for use in collision systems
// Iterates through all regions yielding them along with half of their neighbors

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
