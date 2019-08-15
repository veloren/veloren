use common::{
    terrain::{Block, TerrainChunk, TerrainChunkMeta, TerrainChunkSize, TerrainMap},
    vol::{ReadVol, VolSize, Vox, WriteVol},
};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::thread;
use vek::*;
use world::{sim, ChunkSupplement, World};

fn qser<T: serde::Serialize>(t: PathBuf, obj: &T) -> std::io::Result<()> {
    let out = File::create(t)?;
    bincode::serialize_into(out, obj).unwrap();
    Ok(())
}

fn qdeser<T: serde::de::DeserializeOwned>(t: PathBuf) -> std::io::Result<T> {
    let r = File::open(t)?;
    let val = bincode::deserialize_from(r).unwrap();
    Ok(val)
}

pub enum SaveMsg {
    END,
    SAVE((Vec2<i32>, TerrainChunk)),
    RATE(u32),
}

pub struct Provider {
    pub world: World,
    pub target: PathBuf,

    pub tx: Option<Mutex<mpsc::Sender<SaveMsg>>>,
}

impl Provider {
    pub fn new(seed: u32, target: PathBuf) -> Self {
        let world = Self::load(target.clone()).unwrap_or_else(|_| {
            if target.exists() {
                println!("Failed to open {:?}/, moving to {:?}.old/", target, target);
                std::fs::rename(target.clone(), target.clone().with_extension("old"))
                    .unwrap_or_else(|_| println!("Ok, something strange is happening here..."));
            } else {
                std::fs::create_dir_all(target.clone()).unwrap();
            }
            World::generate(seed)
        });

        Self {
            world,
            target,
            tx: None,
        }
    }

    #[inline(always)]
    pub fn sim(&self) -> &sim::WorldSim {
        self.world.sim()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let t = |val: &str| self.target.join(val);
        qser(t("chunks"), &self.sim().chunks)?;
        qser(t("locations"), &self.sim().locations)?;
        qser(t("seed"), &self.sim().seed)?;

        Ok(())
    }

    pub fn chunk_name(v: Vec2<i32>) -> String {
        format!("{}_{}", v.x, v.y)
    }

    pub fn chunk_path(&self, v: Vec2<i32>) -> PathBuf {
        self.target.join(Self::chunk_name(v))
    }

    pub fn init_save_loop(&mut self) -> thread::JoinHandle<()> {
        let (tx, rx) = mpsc::channel::<SaveMsg>();
        self.tx = Some(Mutex::new(tx));

        let tgt = self.target.clone();
        let t = move |v: Vec2<i32>| tgt.join(Self::chunk_name(v));

        thread::spawn(move || 'yeet: loop {
            let mut wait_time = 1000;
            std::thread::sleep_ms(wait_time);
            for msg in rx.try_recv() {
                match msg {
                    SaveMsg::END => break 'yeet,
                    SaveMsg::RATE(x) => wait_time = x,
                    SaveMsg::SAVE((pos, chunk)) => qser(t(pos), &chunk).unwrap(),
                }
            }
        })
    }

    pub fn request_save_chunk(&self, chunk: TerrainChunk, v: Vec2<i32>) {
        if let Some(mutex) = &self.tx {
            let tx = mutex.lock().unwrap();
            tx.send(SaveMsg::SAVE((v, chunk))).unwrap();
        }
    }

    pub fn save_chunks<T: IntoIterator<Item = Vec2<i32>>>(&self, map: &TerrainMap, chunks: T) {
        let hc: Vec<(Vec2<i32>, TerrainChunk)> = chunks
            .into_iter()
            .map(|pos| (pos, map.get_key(pos).unwrap().clone()))
            .collect();
        let tgt = self.target.clone();
        let t = move |v: Vec2<i32>| tgt.join(Self::chunk_name(v));
        thread::spawn(move || {
            for (pos, chunk) in hc {
                qser(t(pos), &chunk).unwrap();
            }
        });
    }

    pub fn load(target: PathBuf) -> std::io::Result<World> {
        let t = |val: &str| target.join(val);
        let chunks = qdeser(t("chunks"))?;
        let locations = qdeser(t("locations"))?;
        let mut seed = qdeser(t("seed"))?;
        let gen_ctx = sim::GenCtx::from_seed(&mut seed);

        Ok(World {
            sim: sim::WorldSim {
                chunks,
                locations,
                seed,
                gen_ctx,
                rng: sim::get_rng(seed),
            },
        })
    }

    pub fn get_chunk(&self, chunk_pos: Vec2<i32>) -> (TerrainChunk, ChunkSupplement) {
        match qdeser(self.chunk_path(chunk_pos)) {
            Ok(chunk) => (chunk, ChunkSupplement::default()),
            Err(_) => self.world.generate_chunk(chunk_pos),
        }
    }
}
