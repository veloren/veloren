use atomicwrites::{AtomicFile, OverwriteBehavior};
use common::{
    terrain::{Block, TerrainChunk},
    vol::{RectRasterableVol, WriteVol},
};
use hashbrown::HashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    any::{type_name, Any},
    fs::File,
    io::{self, Read as _, Write as _},
    path::PathBuf,
};
use tracing::{debug, error, info, warn};
use vek::*;

pub struct TerrainPersistence {
    path: PathBuf,
    chunks: HashMap<Vec2<i32>, Chunk>,
}

impl TerrainPersistence {
    /// Create a new terrain persistence system using the given data directory.
    ///
    /// If the `VELOREN_TERRAIN` environment variable is set, this will be used
    /// as the persistence directory instead.
    pub fn new(mut data_dir: PathBuf) -> Self {
        let path = std::env::var("VELOREN_TERRAIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                data_dir.push("terrain");
                data_dir
            });

        std::fs::create_dir_all(&path).expect("Failed to create terrain persistence directory");

        info!("Using {:?} as the terrain persistence path", path);

        Self {
            path,
            chunks: HashMap::default(),
        }
    }

    /// Apply persistence changes to a newly generated chunk.
    pub fn apply_changes(&mut self, key: Vec2<i32>, terrain_chunk: &mut TerrainChunk) {
        let chunk = self.load_chunk(key);

        let mut resets = Vec::new();
        for (rpos, new_block) in chunk.blocks() {
            if let Err(e) = terrain_chunk.map(rpos, |block| {
                if block == new_block {
                    resets.push(rpos);
                }
                new_block
            }) {
                warn!(
                    "Could not set block in chunk {:?} with position {:?} (out of bounds?): {:?}",
                    key, rpos, e
                );
            }
        }

        // Reset any unchanged blocks (this is an optimisation only)
        for rpos in resets {
            chunk.reset_block(rpos);
        }
    }

    /// Maintain terrain persistence (writing changes changes back to
    /// filesystem, etc.)
    pub fn maintain(&mut self) {
        // Currently, this does nothing because filesystem writeback occurs on
        // chunk unload However, this is not a particularly reliable
        // mechanism (it doesn't survive power loss, say). Later, a more
        // reliable strategy should be implemented here.
    }

    fn path_for(&self, key: Vec2<i32>) -> PathBuf {
        let mut path = self.path.clone();
        path.push(format!("chunk_{}_{}.dat", key.x, key.y));
        path
    }

    fn load_chunk(&mut self, key: Vec2<i32>) -> &mut Chunk {
        let path = self.path_for(key);
        self.chunks.entry(key).or_insert_with(|| {
            File::open(&path)
                .ok()
                .map(|f| {
                    let bytes = match io::BufReader::new(f).bytes().collect::<Result<Vec<_>, _>>() {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            error!(
                                "Failed to read data for chunk {:?} from file: {:?}",
                                key, err
                            );
                            return Chunk::default();
                        },
                    };
                    match Chunk::deserialize_from(io::Cursor::new(bytes)) {
                        Some(chunk) => chunk,
                        None => {
                            // Find an untaken name for a backup
                            let mut backup_path = path.clone();
                            backup_path.set_extension("dat_backup_0");
                            let mut i = 1;
                            while backup_path.exists() {
                                backup_path.set_extension(format!("dat_backup_{}", i));
                                i += 1;
                            }

                            error!(
                                "Failed to load chunk {:?}, moving possibly corrupt (or too new) \
                                 data to {:?} for you to repair.",
                                key, backup_path
                            );
                            if let Err(err) = std::fs::rename(path, backup_path) {
                                error!("Failed to rename invalid chunk file: {:?}", err);
                            }
                            Chunk::default()
                        },
                    }
                })
                .unwrap_or_default()
        })
    }

    pub fn unload_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk) = self.chunks.remove(&key) {
            // No need to write if no blocks have ever been written
            if chunk.blocks.is_empty() {
                return;
            }

            let bytes = match bincode::serialize::<version::Current>(&chunk.prepare_raw()) {
                Err(err) => {
                    error!("Failed to serialize chunk data: {:?}", err);
                    return;
                },
                Ok(bytes) => bytes,
            };

            let atomic_file =
                AtomicFile::new(self.path_for(key), OverwriteBehavior::AllowOverwrite);
            if let Err(err) = atomic_file.write(|file| file.write_all(&bytes)) {
                error!("Failed to write chunk data to file: {:?}", err);
            }
        }
    }

    pub fn unload_all(&mut self) {
        for key in self.chunks.keys().copied().collect::<Vec<_>>() {
            self.unload_chunk(key);
        }
    }

    pub fn set_block(&mut self, pos: Vec3<i32>, block: Block) {
        let key = pos
            .xy()
            .map2(TerrainChunk::RECT_SIZE, |e, sz| e.div_euclid(sz as i32));
        self.load_chunk(key)
            .blocks
            .insert(pos - key * TerrainChunk::RECT_SIZE.map(|e| e as i32), block);
    }
}

impl Drop for TerrainPersistence {
    fn drop(&mut self) { self.unload_all(); }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Chunk {
    blocks: HashMap<Vec3<i32>, Block>,
}

impl Chunk {
    fn deserialize_from<R: io::Read + Clone>(reader: R) -> Option<Self> {
        version::try_load(reader)
    }

    fn prepare_raw(self) -> version::Current { self.into() }

    fn blocks(&self) -> impl Iterator<Item = (Vec3<i32>, Block)> + '_ {
        self.blocks.iter().map(|(k, b)| (*k, *b))
    }

    fn reset_block(&mut self, rpos: Vec3<i32>) { self.blocks.remove(&rpos); }
}

/// # Adding a new chunk format version
///
/// Chunk formats are designed to be backwards-compatible when loading, but are
/// not required to be backwards-compatible when saving (i.e: we must always be
/// able to load old formats, but we're not required to save old formats because
/// newer formats might contain richer information that is incompatible with an
/// older format).
///
/// The steps for doing this are as follows:
///
/// 1. Create a new 'raw format' type that implements [`Serialize`] and
/// `Deserialize`]. Make sure to add a version field. If in doubt, copy the last
/// raw format and increment the version number wherever it appears. Don't
/// forget to increment the version number in the `serde(deserialize_with =
/// ...}` attribute! Conventionally, these types are named `V{N}` where `{N}` is
/// the number succeeding the previous raw format type.
///
/// 2. Add an implementation of `From<{YourRawFormat}>` for `Chunk`. As before,
/// see previous versions if in doubt.
///
/// 3. Change the type of [`version::Current`] to your new raw format type.
///
/// 4. Add an entry for your raw format at the top of the array in
/// [`version::loaders`].
///
/// 5. Remove the `Serialize` implementation from the previous raw format type:
/// we don't need it any longer!
mod version {
    use super::*;

    /// The newest supported raw format type. This should be changed every time
    /// a new raw format is added.
    // Step [3]
    pub type Current = V3;

    type LoadChunkFn<R> = fn(R) -> Result<Chunk, (&'static str, bincode::Error)>;
    fn loaders<'a, R: io::Read + Clone>() -> &'a [LoadChunkFn<R>] {
        // Step [4]
        &[load_raw::<V3, _>, load_raw::<V2, _>, load_raw::<V1, _>]
    }

    // Convert back to current

    impl From<Chunk> for Current {
        fn from(chunk: Chunk) -> Self {
            Self {
                version: version_magic(3),
                blocks: chunk
                    .blocks
                    .into_iter()
                    .map(|(pos, b)| (pos.x as u8, pos.y as u8, pos.z as i16, b.to_u32()))
                    .collect(),
            }
        }
    }

    /// Version 3 of the raw chunk format.
    #[derive(Serialize, Deserialize)]
    pub struct V3 {
        #[serde(deserialize_with = "version::<_, 3>")]
        pub version: u64,
        pub blocks: Vec<(u8, u8, i16, u32)>,
    }

    impl From<V3> for Chunk {
        fn from(v3: V3) -> Self {
            Self {
                blocks: v3
                    .blocks
                    .into_iter()
                    .map(|(x, y, z, b)| {
                        (
                            Vec3::new(x as i32, y as i32, z as i32),
                            Block::from_u32(b).unwrap_or_else(Block::empty),
                        )
                    })
                    .collect(),
            }
        }
    }

    /// Version 2 of the raw chunk format.
    #[derive(Deserialize)]
    pub struct V2 {
        #[serde(deserialize_with = "version::<_, 2>")]
        pub version: u64,
        pub blocks: Vec<(u8, u8, i16, Block)>,
    }

    impl From<V2> for Chunk {
        fn from(v2: V2) -> Self {
            Self {
                blocks: v2
                    .blocks
                    .into_iter()
                    .map(|(x, y, z, b)| (Vec3::new(x as i32, y as i32, z as i32), b))
                    .collect(),
            }
        }
    }

    /// Version 1 of the raw chunk format.
    #[derive(Deserialize)]
    pub struct V1 {
        pub blocks: HashMap<Vec3<i32>, Block>,
    }

    impl From<V1> for Chunk {
        fn from(v1: V1) -> Self { Self { blocks: v1.blocks } }
    }

    // Utility things

    fn version_magic(n: u16) -> u64 { (n as u64) | (0x3352ACEEA789 << 16) }

    fn version<'de, D: serde::Deserializer<'de>, const V: u16>(de: D) -> Result<u64, D::Error> {
        u64::deserialize(de).and_then(|x| {
            if x == version_magic(V) {
                Ok(x)
            } else {
                Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Unsigned(x),
                    &"incorrect magic/version bytes",
                ))
            }
        })
    }

    fn load_raw<RawChunk: Any + Into<Chunk> + DeserializeOwned, R: io::Read + Clone>(
        reader: R,
    ) -> Result<Chunk, (&'static str, bincode::Error)> {
        bincode::deserialize_from::<_, RawChunk>(reader)
            .map(Into::into)
            .map_err(|e| (type_name::<RawChunk>(), e))
    }

    pub fn try_load<R: io::Read + Clone>(reader: R) -> Option<Chunk> {
        loaders()
            .iter()
            .find_map(|load_raw| match load_raw(reader.clone()) {
                Ok(chunk) => Some(chunk),
                Err((raw_name, e)) => {
                    debug!(
                        "Attempt to load chunk with raw format `{}` failed: {:?}",
                        raw_name, e
                    );
                    None
                },
            })
    }
}
