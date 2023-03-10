use crate::{
    terrain::MapSizeLg,
    util::GridHasher,
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
    /// Size of the entire (not just loaded) map.
    map_size_lg: MapSizeLg,
    /// Default voxel for use outside of max map bounds.
    default: Arc<V>,
    chunks: HashMap<Vec2<i32>, Arc<V>, GridHasher>,
}

impl<V: RectRasterableVol> VolGrid2d<V> {
    #[inline(always)]
    pub fn chunk_key<P: Into<Vec2<i32>>>(pos: P) -> Vec2<i32> {
        pos.into()
            .map2(V::RECT_SIZE, |e, sz: u32| e.div_euclid(sz as i32))
    }

    #[inline(always)]
    pub fn key_chunk<K: Into<Vec2<i32>>>(key: K) -> Vec2<i32> {
        key.into() * V::RECT_SIZE.map(|e| e as i32)
    }

    #[inline(always)]
    pub fn par_keys(&self) -> hashbrown::hash_map::rayon::ParKeys<Vec2<i32>, Arc<V>>
    where
        V: Send + Sync,
    {
        self.chunks.par_keys()
    }

    #[inline(always)]
    pub fn chunk_offs(pos: Vec3<i32>) -> Vec3<i32> {
        let offs = Vec2::<i32>::from(pos).map2(V::RECT_SIZE, |e, sz| e & (sz - 1) as i32);
        Vec3::new(offs.x, offs.y, pos.z)
    }
}

impl<V: RectRasterableVol + Debug> BaseVol for VolGrid2d<V> {
    type Error = VolGrid2dError<V>;
    type Vox = V::Vox;
}

impl<V: RectRasterableVol + ReadVol + Debug> ReadVol for VolGrid2d<V> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V::Vox, VolGrid2dError<V>> {
        let ck = Self::chunk_key(pos);
        self.get_key(ck)
            .ok_or(VolGrid2dError::NoSuchChunk)
            .map(|chunk| {
                let co = Self::chunk_offs(pos);
                // Always within bounds of the chunk, so we can use the get_unchecked form
                chunk.get_unchecked(co)
            })
    }

    /// Call provided closure with each block in the supplied Aabb
    /// Areas outside loaded chunks are ignored
    fn for_each_in(&self, aabb: Aabb<i32>, mut f: impl FnMut(Vec3<i32>, Self::Vox))
    where
        Self::Vox: Copy,
    {
        let min_chunk_key = self.pos_key(aabb.min);
        let max_chunk_key = self.pos_key(aabb.max);
        for key_x in min_chunk_key.x..max_chunk_key.x + 1 {
            for key_y in min_chunk_key.y..max_chunk_key.y + 1 {
                let key = Vec2::new(key_x, key_y);
                let pos = self.key_pos(key);
                // Calculate intersection of Aabb and this chunk
                // TODO: should we do this more implicitly as part of the loop
                // TODO: this probably has to be computed in the chunk.for_each_in() as well
                // maybe remove here?
                let intersection = aabb.intersection(Aabb {
                    min: pos.with_z(i32::MIN),
                    // -1 here since the Aabb is inclusive and chunk_offs below will wrap it if
                    // it's outside the range of the chunk
                    max: (pos + Self::chunk_size().map(|e| e as i32) - 1).with_z(i32::MAX),
                });
                // Map intersection into chunk coordinates
                let intersection = Aabb {
                    min: Self::chunk_offs(intersection.min),
                    max: Self::chunk_offs(intersection.max),
                };
                if let Some(chonk) = self.get_key(key) {
                    chonk.for_each_in(intersection, |pos_offset, block| f(pos_offset + pos, block));
                }
            }
        }
    }
}

// TODO: This actually breaks the API: samples are supposed to have an offset of
// zero! TODO: Should this be changed, perhaps?
impl<I: Into<Aabr<i32>>, V: RectRasterableVol + ReadVol + Debug> SampleVol<I> for VolGrid2d<V> {
    type Sample = VolGrid2d<V>;

    /// Take a sample of the terrain by cloning the voxels within the provided
    /// range.
    ///
    /// Note that the resultant volume does not carry forward metadata from the
    /// original chunks.
    fn sample(&self, range: I) -> Result<Self::Sample, VolGrid2dError<V>> {
        let range = range.into();

        let mut sample = VolGrid2d::new(self.map_size_lg, Arc::clone(&self.default))?;
        let chunk_min = Self::chunk_key(range.min);
        let chunk_max = Self::chunk_key(range.max);
        for x in chunk_min.x..chunk_max.x + 1 {
            for y in chunk_min.y..chunk_max.y + 1 {
                let chunk_key = Vec2::new(x, y);

                let chunk = self.get_key_arc_real(chunk_key).cloned();

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
    fn set(&mut self, pos: Vec3<i32>, vox: V::Vox) -> Result<V::Vox, VolGrid2dError<V>> {
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
    pub fn new(map_size_lg: MapSizeLg, default: Arc<V>) -> Result<Self, VolGrid2dError<V>> {
        if Self::chunk_size()
            .map(|e| e.is_power_of_two() && e > 0)
            .reduce_and()
        {
            Ok(Self {
                map_size_lg,
                default,
                chunks: HashMap::default(),
            })
        } else {
            Err(VolGrid2dError::InvalidChunkSize)
        }
    }

    #[inline(always)]
    pub fn chunk_size() -> Vec2<u32> { V::RECT_SIZE }

    pub fn insert(&mut self, key: Vec2<i32>, chunk: Arc<V>) -> Option<Arc<V>> {
        self.chunks.insert(key, chunk)
    }

    #[inline(always)]
    pub fn get_key(&self, key: Vec2<i32>) -> Option<&V> {
        self.get_key_arc(key).map(|arc_chunk| arc_chunk.as_ref())
    }

    #[inline(always)]
    pub fn get_key_real(&self, key: Vec2<i32>) -> Option<&V> {
        self.get_key_arc_real(key)
            .map(|arc_chunk| arc_chunk.as_ref())
    }

    #[inline(always)]
    pub fn contains_key(&self, key: Vec2<i32>) -> bool {
        self.contains_key_real(key) ||
            // Counterintuitively, areas outside the map are *always* considered to be in it, since
            // they're assigned the default chunk.
            !self.map_size_lg.contains_chunk(key)
    }

    #[inline(always)]
    pub fn contains_key_real(&self, key: Vec2<i32>) -> bool { self.chunks.contains_key(&key) }

    #[inline(always)]
    pub fn get_key_arc(&self, key: Vec2<i32>) -> Option<&Arc<V>> {
        self.get_key_arc_real(key).or_else(|| {
            if !self.map_size_lg.contains_chunk(key) {
                Some(&self.default)
            } else {
                None
            }
        })
    }

    #[inline(always)]
    pub fn get_key_arc_real(&self, key: Vec2<i32>) -> Option<&Arc<V>> { self.chunks.get(&key) }

    pub fn clear(&mut self) { self.chunks.clear(); }

    pub fn drain(&mut self) -> hash_map::Drain<Vec2<i32>, Arc<V>> { self.chunks.drain() }

    pub fn remove(&mut self, key: Vec2<i32>) -> Option<Arc<V>> { self.chunks.remove(&key) }

    /// Converts a chunk key (i.e. coordinates in terms of chunks) into a
    /// position in the world (aka "wpos").
    ///
    /// The returned position will be in the corner of the chunk.
    #[inline(always)]
    pub fn key_pos(&self, key: Vec2<i32>) -> Vec2<i32> { Self::key_chunk(key) }

    /// Converts a position in the world into a chunk key (i.e. coordinates in
    /// terms of chunks).
    #[inline(always)]
    pub fn pos_key(&self, pos: Vec3<i32>) -> Vec2<i32> { Self::chunk_key(pos) }

    /// Gets the chunk that contains the provided world position.
    #[inline(always)]
    pub fn pos_chunk(&self, pos: Vec3<i32>) -> Option<&V> { self.get_key(self.pos_key(pos)) }

    pub fn iter(&self) -> ChunkIter<V> {
        ChunkIter {
            iter: self.chunks.iter(),
        }
    }

    pub fn cached(&self) -> CachedVolGrid2d<V> { CachedVolGrid2d::new(self) }
}

pub struct CachedVolGrid2d<'a, V: RectRasterableVol> {
    vol_grid_2d: &'a VolGrid2d<V>,
    // This can't be invalidated by mutations of the chunks hashmap since we hold an immutable
    // reference to the `VolGrid2d`
    cache: Option<(Vec2<i32>, Arc<V>)>,
}

impl<'a, V: RectRasterableVol> CachedVolGrid2d<'a, V> {
    fn new(vol_grid_2d: &'a VolGrid2d<V>) -> Self {
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
                .get_key_arc(ck)
                .ok_or(VolGrid2dError::NoSuchChunk)?;
            // Store most recently looked up chunk in the cache
            self.cache = Some((ck, Arc::clone(chunk)));
            chunk
        };
        let co = VolGrid2d::<V>::chunk_offs(pos);
        Ok(chunk.get_unchecked(co))
    }
}

impl<'a, V: RectRasterableVol> Deref for CachedVolGrid2d<'a, V> {
    type Target = VolGrid2d<V>;

    fn deref(&self) -> &Self::Target { self.vol_grid_2d }
}

pub struct ChunkIter<'a, V: RectRasterableVol> {
    iter: hash_map::Iter<'a, Vec2<i32>, Arc<V>>,
}

impl<'a, V: RectRasterableVol> Iterator for ChunkIter<'a, V> {
    type Item = (Vec2<i32>, &'a Arc<V>);

    fn next(&mut self) -> Option<Self::Item> { self.iter.next().map(|(k, c)| (*k, c)) }
}
