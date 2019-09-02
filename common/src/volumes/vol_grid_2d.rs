use crate::{
    vol::{BaseVol, ReadVol, RectRasterableVol, SampleVol, VolSize, WriteVol},
    volumes::dyna::DynaError,
};
use hashbrown::{hash_map, HashMap};
use std::marker::PhantomData;
use std::{fmt::Debug, sync::Arc};
use vek::*;

#[derive(Debug, Clone)]
pub enum VolGrid2dError<V: RectRasterableVol> {
    NoSuchChunk,
    ChunkError(V::Error),
    DynaError(DynaError),
    InvalidChunkSize,
}

pub struct VolGrid2dPos<V: RectRasterableVol> {
    key: Vec2<i32>,
    sub_pos: <V as BaseVol>::Pos,
}

impl<V: RectRasterableVol> Clone for VolGrid2dPos<V> {
    fn clone(&self) -> Self {
        VolGrid2dPos::<V> {
            key: self.key,
            sub_pos: self.sub_pos.clone(),
        }
    }
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
    type Pos = VolGrid2dPos<V>;

    fn to_pos(&self, pos: Vec3<i32>) -> Result<Self::Pos, Self::Error> {
        let key = self.pos_key(pos);
        if let Some(chunk) = self.chunks.get(&key) {
            Ok(Self::Pos {
                key,
                sub_pos: chunk
                    .as_ref()
                    .to_pos(pos - Vec3::from(self.key_pos(key)))
                    .unwrap(),
            })
        } else {
            Err(Self::Error::NoSuchChunk)
        }
    }

    fn to_vec3(&self, pos: Self::Pos) -> Vec3<i32> {
        Vec3::from(self.key_pos(pos.key)) + self.chunks[&pos.key].to_vec3(pos.sub_pos)
    }
}

impl<V: RectRasterableVol + ReadVol + Debug> ReadVol for VolGrid2d<V> {
    #[inline(always)]
    fn get_pos(&self, pos: Self::Pos) -> &Self::Vox {
        self.chunks[&pos.key].get_pos(pos.sub_pos)
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
    fn set_pos(&mut self, pos: Self::Pos, vox: V::Vox) {
        // This clones the chunk in case there are other references (clone-on-write).
        Arc::make_mut(self.chunks.get_mut(&pos.key).unwrap()).set_pos(pos.sub_pos, vox);
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
