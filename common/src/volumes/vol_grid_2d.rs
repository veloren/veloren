use crate::{
    vol::{BaseVol, ReadVol, RectRasterableVol, SampleVol, WriteVol},
    volumes::dyna::DynaError,
};
use hashbrown::{hash_map, HashMap};
use std::{fmt::Debug, sync::Arc};
use vek::*;

#[derive(Debug, Clone)]
pub enum VolGrid2dError<V: RectRasterableVol> {
    NoSuchChunk,
    ChunkError(V::Error),
    DynaError(DynaError),
}

// V = Voxel
// S = Size (replace with a const when const generics is a thing)
// M = Chunk metadata
#[derive(Clone)]
pub struct VolGrid2d<V: RectRasterableVol> {
    chunks: HashMap<Vec2<i32>, Arc<V>>,
}

impl<V: RectRasterableVol> VolGrid2d<V> {
    #[inline(always)]
    pub fn chunk_key<P: Into<Vec2<i32>>>(pos: P) -> Vec2<i32> {
        pos.into()
            .map2(V::RECT_SIZE, |e, sz: u32| e >> (sz - 1).count_ones())
    }

    #[inline(always)]
    pub fn chunk_offs(pos: Vec3<i32>) -> Vec3<i32> {
        let offs = Vec2::<i32>::from(pos).map2(V::RECT_SIZE, |e, sz| e & (sz - 1) as i32);
        Vec3::new(offs.x, offs.y, pos.z)
    }
}

impl<V: RectRasterableVol + Debug> BaseVol for VolGrid2d<V> {
    type Vox = V::Vox;
    type Error = VolGrid2dError<V>;
}

impl<V: RectRasterableVol + ReadVol + Debug> ReadVol for VolGrid2d<V> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, Self::Error> {
        let ck = Self::chunk_key(pos);
        self.chunks
            .get(&ck)
            .ok_or(VolGrid2dError::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                chunk.get(co).map_err(VolGrid2dError::ChunkError)
            })
    }
}

// TODO: This actually breaks the API: samples are supposed to have an offset of zero!
// TODO: Should this be changed, perhaps?
impl<I: Into<Aabr<i32>>, V: RectRasterableVol + ReadVol + Debug> SampleVol<I> for VolGrid2d<V> {
    type Sample = VolGrid2d<V>;

    /// Take a sample of the terrain by cloning the voxels within the provided range.
    ///
    /// Note that the resultant volume does not carry forward metadata from the original chunks.
    fn sample(&self, range: I) -> Result<Self::Sample, VolGrid2dError<V>> {
        let range = range.into();

        let mut sample = VolGrid2d::new();
        let chunk_min = Self::chunk_key(range.min);
        let chunk_max = Self::chunk_key(range.max);
        for x in chunk_min.x..=chunk_max.x {
            for y in chunk_min.y..=chunk_max.y {
                let chunk_key = Vec2::new(x, y);

                let chunk = self.get_key_arc(chunk_key).cloned();

                if let Some(chunk) = chunk {
                    sample.insert(chunk_key, chunk);
                }
            }
        }

        Ok(sample)
    }
}

impl<V: RectRasterableVol + WriteVol + Clone + Debug> WriteVol for VolGrid2d<V> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: V::Vox) -> Result<(), VolGrid2dError<V>> {
        let ck = Self::chunk_key(pos);
        self.chunks
            .get_mut(&ck)
            .ok_or(VolGrid2dError::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                Arc::make_mut(chunk)
                    .set(co, vox)
                    .map_err(VolGrid2dError::ChunkError)
            })
    }
}

impl<V: RectRasterableVol> VolGrid2d<V> {
    pub fn new() -> Self {
        // TODO (haslersn): Turn this into a compile time assertion.
        assert!(V::RECT_SIZE
            .map(|e| e.is_power_of_two() && e > 0)
            .reduce_and());
        Self {
            chunks: HashMap::default(),
        }
    }

    pub fn chunk_size() -> Vec2<u32> {
        V::RECT_SIZE
    }

    pub fn insert(&mut self, key: Vec2<i32>, chunk: Arc<V>) -> Option<Arc<V>> {
        self.chunks.insert(key, chunk)
    }

    pub fn get_key(&self, key: Vec2<i32>) -> Option<&V> {
        match self.chunks.get(&key) {
            Some(arc_chunk) => Some(arc_chunk.as_ref()),
            None => None,
        }
    }

    pub fn get_key_arc(&self, key: Vec2<i32>) -> Option<&Arc<V>> {
        self.chunks.get(&key)
    }

    pub fn get_key_arc_mut(&mut self, key: Vec2<i32>) -> Option<&mut Arc<V>> {
        self.chunks.get_mut(&key)
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    pub fn drain(&mut self) -> hash_map::Drain<Vec2<i32>, Arc<V>> {
        self.chunks.drain()
    }

    pub fn remove(&mut self, key: Vec2<i32>) -> Option<Arc<V>> {
        self.chunks.remove(&key)
    }

    pub fn key_pos(&self, key: Vec2<i32>) -> Vec2<i32> {
        key * V::RECT_SIZE.map(|e| e as i32)
    }

    pub fn pos_key(&self, pos: Vec3<i32>) -> Vec2<i32> {
        Self::chunk_key(pos)
    }

    pub fn iter(&self) -> ChunkIter<V> {
        ChunkIter {
            iter: self.chunks.iter(),
        }
    }
}

#[derive(Clone)]
pub enum VolGrid2dChange<V: RectRasterableVol + ReadVol + WriteVol + Clone> {
    Insert(Arc<V>),
    Remove,
}

#[derive(Clone)]
pub struct VolGrid2dJournal<V: RectRasterableVol + ReadVol + WriteVol + Clone> {
    grid: VolGrid2d<V>,
    requested_changes: HashMap<Vec2<i32>, VolGrid2dChange<V>>,
    requested_vox_changes: HashMap<Vec3<i32>, V::Vox>,
    previous_changes: HashMap<Vec2<i32>, VolGrid2dChange<V>>,
    previous_vox_changes: HashMap<Vec3<i32>, V::Vox>,
}

impl<V: RectRasterableVol + ReadVol + WriteVol + Clone> VolGrid2dJournal<V> {
    pub fn new() -> Self {
        Self {
            grid: VolGrid2d::<V>::new(),
            requested_changes: Default::default(),
            requested_vox_changes: Default::default(),
            previous_changes: Default::default(),
            previous_vox_changes: Default::default(),
        }
    }

    fn foreseen_chunk(&self, key: Vec2<i32>) -> Option<&Arc<V>> {
        self.requested_changes.get(&key).map_or_else(
            || self.grid.get_key_arc(key),
            |c| {
                if let VolGrid2dChange::<V>::Insert(c) = c {
                    Some(c)
                } else {
                    None
                }
            },
        )
    }

    fn foreseen_chunk_mut(&mut self, key: Vec2<i32>) -> Option<&mut Arc<V>> {
        if let Some(change) = self.requested_changes.get_mut(&key) {
            if let VolGrid2dChange::<V>::Insert(chunk) = change {
                Some(chunk)
            } else {
                None
            }
        } else {
            self.grid.get_key_arc_mut(key)
        }
    }

    pub fn request_change(&mut self, key: Vec2<i32>, change: VolGrid2dChange<V>) {
        if let VolGrid2dChange::<V>::Insert(ref chunk) = change {
            if self
                .foreseen_chunk(key)
                .map_or(true, |c| !Arc::ptr_eq(c, chunk))
            {
                self.requested_changes.insert(key, change);
            }
        } else if self.grid.get_key(key).is_some() {
            self.requested_changes.insert(key, change);
        } else {
            self.requested_changes.remove(&key);
        }
    }

    pub fn request_vox_change(&mut self, pos: Vec3<i32>, vox: V::Vox) {
        let key = VolGrid2d::<V>::chunk_key(pos);
        let offs = VolGrid2d::<V>::chunk_offs(pos);
        if let Some(chunk) = self.foreseen_chunk_mut(key) {
            let old_vox = chunk.get(offs).unwrap();
            if *old_vox != vox {
                // Can't fail because we got the `offs` from `VolGrid2d::<V>::chunk_offs`.
                Arc::make_mut(chunk).set(offs, vox.clone()).unwrap();
                self.requested_vox_changes.insert(pos, vox);
            }
        } else {
            // TODO (haslersn): Error! What to do?
        }
    }

    pub fn request_clearance(&mut self) {
        self.requested_changes.clear();
        self.requested_vox_changes.clear();
        for (key, _) in self.grid.iter() {
            self.requested_changes
                .insert(key, VolGrid2dChange::<V>::Remove);
        }
    }

    pub fn apply(&mut self) {
        for (&key, change) in &self.requested_changes {
            if let VolGrid2dChange::<V>::Insert(chunk) = change {
                self.grid.insert(key, chunk.clone()); // Clones the `Arc`.
            } else {
                self.grid.remove(key);
            }
        }
        self.previous_changes.clear();
        self.previous_vox_changes.clear();
        // Swap them as in a double buffer.
        std::mem::swap(&mut self.requested_changes, &mut self.previous_changes);
        std::mem::swap(
            &mut self.requested_vox_changes,
            &mut self.previous_vox_changes,
        );
    }

    pub fn grid(&self) -> &VolGrid2d<V> {
        &self.grid
    }

    pub fn previous_changes(&self) -> &HashMap<Vec2<i32>, VolGrid2dChange<V>> {
        &self.previous_changes
    }

    pub fn previous_vox_changes(&self) -> &HashMap<Vec3<i32>, V::Vox> {
        &self.previous_vox_changes
    }
}

pub struct ChunkIter<'a, V: RectRasterableVol> {
    iter: hash_map::Iter<'a, Vec2<i32>, Arc<V>>,
}

impl<'a, V: RectRasterableVol> Iterator for ChunkIter<'a, V> {
    type Item = (Vec2<i32>, &'a Arc<V>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, c)| (*k, c))
    }
}
