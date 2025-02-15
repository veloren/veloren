pub mod event;
pub mod rule;
pub mod tick;

use atomicwrites::{AtomicFile, OverwriteBehavior};
use common::{
    grid::Grid,
    mounting::VolumePos,
    rtsim::{Actor, ChunkResource, NpcId, RtSimEntity, WorldSettings},
    terrain::{CoordinateConversions, SpriteKind},
};
use common_ecs::{System, dispatch};
use common_state::BlockDiff;
use crossbeam_channel::{Receiver, Sender, unbounded};
use enum_map::EnumMap;
use rtsim::{
    RtState,
    data::{Data, ReadError, npc::SimulationMode},
    event::{OnDeath, OnHealthChange, OnMountVolume, OnSetup, OnTheft},
};
use specs::DispatcherBuilder;
use std::{
    fs::{self, File},
    io,
    path::PathBuf,
    thread::{self, JoinHandle},
    time::Instant,
};
use tracing::{debug, error, info, trace, warn};
use vek::*;
use world::{IndexRef, World};

pub struct RtSim {
    file_path: PathBuf,
    last_saved: Option<Instant>,
    state: RtState,
    save_thread: Option<(Sender<Data>, JoinHandle<()>)>,
}

impl RtSim {
    pub fn new(
        settings: &WorldSettings,
        index: IndexRef,
        world: &World,
        data_dir: PathBuf,
    ) -> Result<Self, ron::Error> {
        let file_path = Self::get_file_path(data_dir);

        info!("Looking for rtsim data at {}...", file_path.display());
        let data = 'load: {
            if std::env::var("RTSIM_NOLOAD").map_or(true, |v| v != "1") {
                match File::open(&file_path) {
                    Ok(file) => {
                        info!("Rtsim data found. Attempting to load...");

                        let ignore_version = std::env::var("RTSIM_IGNORE_VERSION").is_ok();

                        match Data::from_reader(io::BufReader::new(file)) {
                            Err(ReadError::VersionMismatch(_)) if !ignore_version => {
                                warn!(
                                    "Rtsim data version mismatch (implying a breaking change), \
                                     rtsim data will be purged"
                                );
                            },
                            Ok(data) | Err(ReadError::VersionMismatch(data)) => {
                                info!("Rtsim data loaded.");
                                if data.should_purge {
                                    warn!(
                                        "The should_purge flag was set on the rtsim data, \
                                         generating afresh"
                                    );
                                } else {
                                    break 'load *data;
                                }
                            },
                            Err(ReadError::Load(err)) => {
                                error!("Rtsim data failed to load: {}", err);
                                info!("Old rtsim data will now be moved to a backup file");
                                let mut i = 0;
                                loop {
                                    let mut backup_path = file_path.clone();
                                    backup_path.set_extension(if i == 0 {
                                        "ron_backup".to_string()
                                    } else {
                                        format!("ron_backup_{}", i)
                                    });
                                    if !backup_path.exists() {
                                        fs::rename(&file_path, &backup_path)?;
                                        warn!(
                                            "Failed rtsim data was moved to {}",
                                            backup_path.display()
                                        );
                                        info!("A fresh rtsim data will now be generated.");
                                        break;
                                    } else {
                                        info!(
                                            "Backup file {} already exists, trying another name...",
                                            backup_path.display()
                                        );
                                    }
                                    i += 1;
                                }
                            },
                        }
                    },
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        info!("No rtsim data found. Generating from world...")
                    },
                    Err(e) => return Err(e.into()),
                }
            } else {
                warn!(
                    "'RTSIM_NOLOAD' is set, skipping loading of rtsim state (old state will be \
                     overwritten)."
                );
            }

            let data = Data::generate(settings, world, index);
            info!("Rtsim data generated.");
            data
        };

        let mut this = Self {
            last_saved: None,
            state: RtState::new(data).with_resource(ChunkStates(Grid::populate_from(
                world.sim().get_size().as_(),
                |_| None,
            ))),
            file_path,
            save_thread: None,
        };

        rule::start_rules(&mut this.state);

        this.state.emit(OnSetup, &mut (), world, index);

        Ok(this)
    }

    fn get_file_path(mut data_dir: PathBuf) -> PathBuf {
        let mut path = std::env::var("VELOREN_RTSIM")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                data_dir.push("rtsim");
                data_dir
            });
        path.push("data.dat");
        path
    }

    pub fn hook_character_mount_volume(
        &mut self,
        world: &World,
        index: IndexRef,
        pos: VolumePos<NpcId>,
        actor: Actor,
    ) {
        self.state
            .emit(OnMountVolume { actor, pos }, &mut (), world, index)
    }

    pub fn hook_pickup_owned_sprite(
        &mut self,
        world: &World,
        index: IndexRef,
        sprite: SpriteKind,
        wpos: Vec3<i32>,
        actor: Actor,
    ) {
        let site = world.sim().get(wpos.xy().wpos_to_cpos()).and_then(|chunk| {
            chunk
                .sites
                .iter()
                .find_map(|site| self.state.data().sites.world_site_map.get(site).copied())
        });

        self.state.emit(
            OnTheft {
                actor,
                wpos,
                sprite,
                site,
            },
            &mut (),
            world,
            index,
        )
    }

    pub fn hook_load_chunk(&mut self, key: Vec2<i32>, max_res: EnumMap<ChunkResource, usize>) {
        if let Some(chunk_state) = self.state.get_resource_mut::<ChunkStates>().0.get_mut(key) {
            *chunk_state = Some(LoadedChunkState { max_res });
        }
    }

    pub fn hook_unload_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk_state) = self.state.get_resource_mut::<ChunkStates>().0.get_mut(key) {
            *chunk_state = None;
        }
    }

    // Note that this hook only needs to be invoked if the block change results in a
    // change to the rtsim resource produced by [`Block::get_rtsim_resource`].
    pub fn hook_block_update(&mut self, world: &World, index: IndexRef, changes: Vec<BlockDiff>) {
        self.state
            .emit(event::OnBlockChange { changes }, &mut (), world, index);
    }

    pub fn hook_rtsim_entity_unload(&mut self, entity: RtSimEntity) {
        let data = self.state.get_data_mut();

        if let Some(npc) = data.npcs.get_mut(entity.0) {
            if matches!(npc.mode, SimulationMode::Simulated) {
                error!("Unloaded already unloaded entity");
            }
            npc.mode = SimulationMode::Simulated;
        }
    }

    pub fn hook_rtsim_actor_hp_change(
        &mut self,
        world: &World,
        index: IndexRef,
        actor: Actor,
        cause: Option<Actor>,
        new_hp_fraction: f32,
    ) {
        self.state.emit(
            OnHealthChange {
                actor,
                cause,
                new_health_fraction: new_hp_fraction,
            },
            &mut (),
            world,
            index,
        )
    }

    pub fn hook_rtsim_actor_death(
        &mut self,
        world: &World,
        index: IndexRef,
        actor: Actor,
        wpos: Option<Vec3<f32>>,
        killer: Option<Actor>,
    ) {
        self.state.emit(
            OnDeath {
                wpos,
                actor,
                killer,
            },
            &mut (),
            world,
            index,
        );
    }

    pub fn save(&mut self, wait_until_finished: bool) {
        debug!("Saving rtsim data...");

        // Create the save thread if it doesn't already exist
        // We're not using the slow job pool here for two reasons:
        // 1) The thread is mostly blocked on IO, not compute
        // 2) We need to synchronise saves to ensure monotonicity, which slow jobs
        // aren't designed to allow
        let (tx, _) = self.save_thread.get_or_insert_with(|| {
            trace!("Starting rtsim data save thread...");
            let (tx, rx) = unbounded();
            let file_path = self.file_path.clone();
            (tx, thread::spawn(move || save_thread(file_path, rx)))
        });

        // Send rtsim data to the save thread
        if let Err(err) = tx.send(self.state.data().clone()) {
            error!("Failed to perform rtsim save: {}", err);
        }

        // If we need to wait until the save thread has done its work (due to, for
        // example, server shutdown) then do that.
        if wait_until_finished {
            if let Some((tx, handle)) = self.save_thread.take() {
                drop(tx);
                info!("Waiting for rtsim save thread to finish...");
                handle.join().expect("Save thread failed to join");
                info!("Rtsim save thread finished.");
            }
        }

        self.last_saved = Some(Instant::now());
    }

    // TODO: Clean up this API a bit
    pub fn get_chunk_resources(&self, key: Vec2<i32>) -> EnumMap<ChunkResource, f32> {
        self.state.data().nature.get_chunk_resources(key)
    }

    pub fn state(&self) -> &RtState { &self.state }

    pub fn set_should_purge(&mut self, should_purge: bool) {
        self.state.data_mut().should_purge = should_purge;
    }
}

fn save_thread(file_path: PathBuf, rx: Receiver<Data>) {
    if let Some(dir) = file_path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    let atomic_file = AtomicFile::new(file_path, OverwriteBehavior::AllowOverwrite);
    while let Ok(data) = rx.recv() {
        debug!("Writing rtsim data to file...");
        match atomic_file.write(move |file| data.write_to(io::BufWriter::new(file))) {
            Ok(_) => debug!("Rtsim data saved."),
            Err(e) => error!("Saving rtsim data failed: {}", e),
        }
    }
}

pub struct ChunkStates(pub Grid<Option<LoadedChunkState>>);

pub struct LoadedChunkState {
    // The maximum possible number of each resource in this chunk
    pub max_res: EnumMap<ChunkResource, usize>,
}

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<tick::Sys>(dispatch_builder, &[&common_systems::phys::Sys::sys_name()]);
}
