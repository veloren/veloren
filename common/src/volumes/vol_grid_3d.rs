use crate::{
    vol::{BaseVol, RasterableVol, ReadVol, SampleVol, WriteVol},
    volumes::dyna::DynaError,
};
use hashbrown::{hash_map, HashMap};
use std::{fmt::Debug, sync::Arc};
use vek::*;

#[derive(Debug)]
pub enum VolGrid3dError<V: RasterableVol> {
    NoSuchChunk,
    ChunkErr(V::Error),
    DynaError(DynaError),
    InvalidChunkSize,
}

// V = Voxel
// S = Size (replace with a const when const generics is a thing)
// M = Chunk metadata
#[derive(Clone)]
pub struct VolGrid3d<V: RasterableVol> {
    chunks: HashMap<Vec3<i32>, Arc<V>>,
}

impl<V: RasterableVol> VolGrid3d<V> {
    #[inline(always)]
    pub fn chunk_key(pos: Vec3<i32>) -> Vec3<i32> {
        pos.map2(V::SIZE, |e, sz| {
            // Horrid, but it's faster than a cheetah with a red bull blood transfusion
            let log2 = (sz - 1).count_ones();
            ((((i64::from(e) + (1 << 32)) as u64) >> log2) - (1 << (32 - log2))) as i32
        })
    }

    #[inline(always)]
    pub fn chunk_offs(pos: Vec3<i32>) -> Vec3<i32> {
        pos.map2(V::SIZE, |e, sz| {
            // Horrid, but it's even faster than the aforementioned cheetah
            (((i64::from(e) + (1 << 32)) as u64) & u64::from(sz - 1)) as i32
        })
    }
}

impl<V: RasterableVol + Debug> BaseVol for VolGrid3d<V> {
    type Vox = V::Vox;
    type Error = VolGrid3dError<V>;
}

impl<V: RasterableVol + ReadVol + Debug> ReadVol for VolGrid3d<V> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V::Vox, VolGrid3dError<V>> {
        let ck = Self::chunk_key(pos);
        self.chunks
            .get(&ck)
            .ok_or(VolGrid3dError::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                chunk.get(co).map_err(VolGrid3dError::ChunkErr)
            })
    }
}

// TODO: This actually breaks the API: samples are supposed to have an offset of zero!
// TODO: Should this be changed, perhaps?
impl<I: Into<Aabb<i32>>, V: RasterableVol + ReadVol + Debug> SampleVol<I> for VolGrid3d<V> {
    type Sample = VolGrid3d<V>;

    /// Take a sample of the terrain by cloning the voxels within the provided range.
    ///
    /// Note that the resultant volume does not carry forward metadata from the original chunks.
    fn sample(&self, range: I) -> Result<Self::Sample, VolGrid3dError<V>> {
        let range = range.into();
        let mut sample = VolGrid3d::new()?;
        let chunk_min = Self::chunk_key(range.min);
        let chunk_max = Self::chunk_key(range.max);
        for x in chunk_min.x..chunk_max.x + 1 {
            for y in chunk_min.y..chunk_max.y + 1 {
                for z in chunk_min.z..chunk_max.z + 1 {
                    let chunk_key = Vec3::new(x, y, z);

                    let chunk = self.get_key_arc(chunk_key).cloned();

                    if let Some(chunk) = chunk {
                        sample.insert(chunk_key, chunk);
                    }
                }
            }
        }

        Ok(sample)
    }
}

impl<V: RasterableVol + WriteVol + Clone + Debug> WriteVol for VolGrid3d<V> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: V::Vox) -> Result<(), VolGrid3dError<V>> {
        let ck = Self::chunk_key(pos);
        self.chunks
            .get_mut(&ck)
            .ok_or(VolGrid3dError::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                Arc::make_mut(chunk)
                    .set(co, vox)
                    .map_err(VolGrid3dError::ChunkErr)
            })
    }
}

impl<V: RasterableVol> VolGrid3d<V> {
    pub fn new() -> Result<Self, VolGrid3dError<V>> {
        if Self::chunk_size()
            .map(|e| e.is_power_of_two() && e > 0)
            .reduce_and()
        {
            Ok(Self {
                chunks: HashMap::new(),
            })
        } else {
            Err(VolGrid3dError::InvalidChunkSize)
        }
    }

    pub fn chunk_size() -> Vec3<u32> {
        V::SIZE
    }

    pub fn insert(&mut self, key: Vec3<i32>, chunk: Arc<V>) -> Option<Arc<V>> {
        self.chunks.insert(key, chunk)
    }

    pub fn get_key(&self, key: Vec3<i32>) -> Option<&V> {
        match self.chunks.get(&key) {
            Some(arc_chunk) => Some(arc_chunk.as_ref()),
            None => None,
        }
    }

    pub fn get_key_arc(&self, key: Vec3<i32>) -> Option<&Arc<V>> {
        self.chunks.get(&key)
    }

    pub fn remove(&mut self, key: Vec3<i32>) -> Option<Arc<V>> {
        self.chunks.remove(&key)
    }

    pub fn key_pos(&self, key: Vec3<i32>) -> Vec3<i32> {
        key * V::SIZE.map(|e| e as i32)
    }

    pub fn pos_key(&self, pos: Vec3<i32>) -> Vec3<i32> {
        Self::chunk_key(pos)
    }

    pub fn iter(&self) -> ChunkIter<V> {
        ChunkIter {
            iter: self.chunks.iter(),
        }
    }
}

pub struct ChunkIter<'a, V: RasterableVol> {
    iter: hash_map::Iter<'a, Vec3<i32>, Arc<V>>,
}

impl<'a, V: RasterableVol> Iterator for ChunkIter<'a, V> {
    type Item = (Vec3<i32>, &'a Arc<V>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, c)| (*k, c))
    }
}
