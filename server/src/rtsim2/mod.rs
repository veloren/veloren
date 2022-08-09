pub mod tick;

use common::{
    grid::Grid,
    slowjob::SlowJobPool,
    rtsim::ChunkResource,
    terrain::{TerrainChunk, Block},
    vol::RectRasterableVol,
};
use common_ecs::{dispatch, System};
use rtsim2::{data::{Data, ReadError}, RtState};
use specs::{DispatcherBuilder, WorldExt};
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
    time::Instant,
    io::{self, Write},
    error::Error,
};
use enum_map::EnumMap;
use tracing::{error, warn, info};
use vek::*;
use world::World;

pub struct RtSim {
    file_path: PathBuf,
    last_saved: Option<Instant>,
    chunk_states: Grid<Option<LoadedChunkState>>,
    state: RtState,
}

impl RtSim {
    pub fn new(world: &World, data_dir: PathBuf) -> Result<Self, ron::Error> {
        let file_path = Self::get_file_path(data_dir);

        Ok(Self {
            chunk_states: Grid::populate_from(world.sim().get_size().as_(), |_| None),
            last_saved: None,
            state: RtState {
                data: {
                    info!("Looking for rtsim state in {}...", file_path.display());
                    'load: {
                        match File::open(&file_path) {
                            Ok(file) => {
                                info!("Rtsim state found. Attempting to load...");
                                match Data::from_reader(io::BufReader::new(file)) {
                                    Ok(data) => { info!("Rtsim state loaded."); break 'load data },
                                    Err(e) => {
                                        error!("Rtsim state failed to load: {}", e);
                                        let mut i = 0;
                                        loop {
                                            let mut backup_path = file_path.clone();
                                            backup_path.set_extension(if i == 0 {
                                                format!("backup_{}", i)
                                            } else {
                                                "ron_backup".to_string()
                                            });
                                            if !backup_path.exists() {
                                                fs::rename(&file_path, &backup_path)?;
                                                warn!("Failed rtsim state was moved to {}", backup_path.display());
                                                info!("A fresh rtsim state will now be generated.");
                                                break;
                                            }
                                            i += 1;
                                        }
                                    },
                                }
                            },
                            Err(e) if e.kind() == io::ErrorKind::NotFound =>
                                info!("No rtsim state found. Generating from initial world state..."),
                            Err(e) => return Err(e.into()),
                        }

                        let data = Data::generate(&world);
                        info!("Rtsim state generated.");
                        data
                    }
                },
            },
            file_path,
        })
    }

    fn get_file_path(mut data_dir: PathBuf) -> PathBuf {
        let mut path = std::env::var("VELOREN_RTSIM")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                data_dir.push("rtsim");
                data_dir
            });
        path.push("state.dat");
        path
    }

    pub fn hook_load_chunk(&mut self, key: Vec2<i32>, max_res: EnumMap<ChunkResource, usize>) {
        if let Some(chunk_state) = self.chunk_states.get_mut(key) {
            *chunk_state = Some(LoadedChunkState { max_res });
        }
    }

    pub fn hook_unload_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk_state) = self.chunk_states.get_mut(key) {
            *chunk_state = None;
        }
    }

    pub fn save(&mut self, slowjob_pool: &SlowJobPool) {
        info!("Beginning rtsim state save...");
        let file_path = self.file_path.clone();
        let data = self.state.data.clone();
        info!("Starting rtsim save job...");
        // TODO: Use slow job
        // slowjob_pool.spawn("RTSIM_SAVE", move || {
        std::thread::spawn(move || {
            let tmp_file_name = "state_tmp.dat";
            if let Err(e) = file_path
                .parent()
                .map(|dir| {
                    fs::create_dir_all(dir)?;
                    // We write to a temporary file and then rename to avoid corruption.
                    Ok(dir.join(tmp_file_name))
                })
                .unwrap_or_else(|| Ok(tmp_file_name.into()))
                .and_then(|tmp_file_path| {
                    Ok((File::create(&tmp_file_path)?, tmp_file_path))
                })
                .map_err(|e: io::Error| Box::new(e) as Box::<dyn Error>)
                .and_then(|(mut file, tmp_file_path)| {
                    info!("Writing rtsim state to file...");
                    data.write_to(io::BufWriter::new(&mut file))?;
                    file.flush()?;
                    drop(file);
                    fs::rename(tmp_file_path, file_path)?;
                    info!("Rtsim state saved.");
                    Ok(())
                })
            {
                error!("Saving rtsim state failed: {}", e);
            }
        });
        self.last_saved = Some(Instant::now());
    }

    // TODO: Clean up this API a bit
    pub fn get_chunk_resources(&self, key: Vec2<i32>) -> EnumMap<ChunkResource, f32> {
        self.state.data.nature.get_chunk_resources(key)
    }
    pub fn hook_block_update(&mut self, wpos: Vec3<i32>, old_block: Block, new_block: Block) {
        let key = wpos
            .xy()
            .map2(TerrainChunk::RECT_SIZE, |e, sz| e.div_euclid(sz as i32));
        if let Some(Some(chunk_state)) = self.chunk_states.get(key) {
            let mut chunk_res = self.get_chunk_resources(key);
            // Remove resources
            if let Some(res) = old_block.get_rtsim_resource() {
                if chunk_state.max_res[res] > 0 {
                    chunk_res[res] = (chunk_res[res] - 1.0 / chunk_state.max_res[res] as f32).max(0.0);
                    println!("Subbing {} to resources", 1.0 / chunk_state.max_res[res] as f32);
                }
            }
            // Add resources
            if let Some(res) = new_block.get_rtsim_resource() {
                if chunk_state.max_res[res] > 0 {
                    chunk_res[res] = (chunk_res[res] + 1.0 / chunk_state.max_res[res] as f32).min(1.0);
                    println!("Added {} to resources", 1.0 / chunk_state.max_res[res] as f32);
                }
            }
            println!("Chunk resources are {:?}", chunk_res);
            self.state.data.nature.set_chunk_resources(key, chunk_res);
        }
    }
}

struct LoadedChunkState {
    // The maximum possible number of each resource in this chunk
    max_res: EnumMap<ChunkResource, usize>,
}

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<tick::Sys>(dispatch_builder, &[]);
}
