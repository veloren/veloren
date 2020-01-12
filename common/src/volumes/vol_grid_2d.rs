use crate::{
    vol::{BaseVol, ReadVol, RectRasterableVol, SampleVol, WriteVol},
    volumes::dyna::DynaError,
};
use hashbrown::{hash_map, HashMap};
use std::{fmt::Debug, ops::Deref, sync::Arc};
use vek::*;

#[derive(Debug, Clone)]
pub enum VolGrid2dError<V: RectRasterableVol> {
    NoSuchChunk,
    ChunkError(V::Error),
    DynaError(DynaError),
    InvalidChunkSize,
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
    fn get(&self, pos: Vec3<i32>) -> Result<&V::Vox, VolGrid2dError<V>> {
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

        let mut sample = VolGrid2d::new()?;
        let chunk_min = Self::chunk_key(range.min);
        let chunk_max = Self::chunk_key(range.max);
        for x in chunk_min.x..chunk_max.x + 1 {
            for y in chunk_min.y..chunk_max.y + 1 {
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
    pub fn new() -> Result<Self, VolGrid2dError<V>> {
        if Self::chunk_size()
            .map(|e| e.is_power_of_two() && e > 0)
            .reduce_and()
        {
            Ok(Self {
                chunks: HashMap::default(),
            })
        } else {
            Err(VolGrid2dError::InvalidChunkSize)
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

    pub fn cached<'a>(&'a self) -> CachedVolGrid2d<'a, V> {
        CachedVolGrid2d::new(self)
    }
}

pub struct CachedVolGrid2d<'a, V: RectRasterableVol> {
    vol_grid_2d: &'a VolGrid2d<V>,
    // This can't be invalidated by mutations of the chunks hashmap since we hold an immutable
    // reference to the `VolGrid2d`
    cache: Option<(Vec2<i32>, Arc<V>)>,
}
impl<'a, V: RectRasterableVol> CachedVolGrid2d<'a, V> {
    pub fn new(vol_grid_2d: &'a VolGrid2d<V>) -> Self {
        Self {
            vol_grid_2d,
            cache: None,
        }
    }
}
impl<'a, V: RectRasterableVol + ReadVol> CachedVolGrid2d<'a, V> {
    #[inline(always)]
    pub fn get(&mut self, pos: Vec3<i32>) -> Result<&V::Vox, VolGrid2dError<V>> {
        // Calculate chunk key from block pos
        let ck = VolGrid2d::<V>::chunk_key(pos);
        let chunk = if self
            .cache
            .as_ref()
            .map(|(key, _)| *key == ck)
            .unwrap_or(false)
        {
            // If the chunk with that key is in the cache use that
            &self.cache.as_ref().unwrap().1
        } else {
            // Otherwise retrieve from the hashmap
            let chunk = self
                .vol_grid_2d
                .chunks
                .get(&ck)
                .ok_or(VolGrid2dError::NoSuchChunk)?;
            // Store most recently looked up chunk in the cache
            self.cache = Some((ck, chunk.clone()));
            chunk
        };
        let co = VolGrid2d::<V>::chunk_offs(pos);
        chunk.get(co).map_err(VolGrid2dError::ChunkError)
    }
}

impl<'a, V: RectRasterableVol> Deref for CachedVolGrid2d<'a, V> {
    type Target = VolGrid2d<V>;

    fn deref(&self) -> &Self::Target {
        self.vol_grid_2d
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
