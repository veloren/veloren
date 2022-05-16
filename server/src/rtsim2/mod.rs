pub mod tick;

use common::grid::Grid;
use common_ecs::{dispatch, System};
use rtsim2::{data::Data, RtState};
use specs::{DispatcherBuilder, WorldExt};
use std::{fs::File, io, path::PathBuf, sync::Arc};
use tracing::info;
use vek::*;
use world::World;

pub struct RtSim {
    file_path: PathBuf,
    chunk_states: Grid<bool>, // true = loaded
    state: RtState,
}

impl RtSim {
    pub fn new(world: &World, data_dir: PathBuf) -> Result<Self, ron::Error> {
        let file_path = Self::get_file_path(data_dir);

        Ok(Self {
            chunk_states: Grid::populate_from(world.sim().get_size().as_(), |_| false),
            state: RtState {
                data: {
                    info!("Looking for rtsim state in {}...", file_path.display());
                    match File::open(&file_path) {
                        Ok(file) => {
                            info!("Rtsim state found. Attending to load...");
                            Data::from_reader(file)?
                        },
                        Err(e) if e.kind() == io::ErrorKind::NotFound => {
                            info!("No rtsim state found. Generating from initial world state...");
                            Data::generate(&world)
                        },
                        Err(e) => return Err(e.into()),
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
        path.push("state.ron");
        path
    }

    pub fn hook_load_chunk(&mut self, key: Vec2<i32>) {
        if let Some(is_loaded) = self.chunk_states.get_mut(key) {
            *is_loaded = true;
        }
    }

    pub fn hook_unload_chunk(&mut self, key: Vec2<i32>) {
        if let Some(is_loaded) = self.chunk_states.get_mut(key) {
            *is_loaded = false;
        }
    }
}

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<tick::Sys>(dispatch_builder, &[]);
}
