use common::terrain::TerrainChunk;
//use std::collections::HashMap;
use crossbeam::channel;
use flate2::{bufread::DeflateDecoder, write::DeflateEncoder, Compression};
use log;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::thread;
use vek::*;
use world::{sim, ChunkSupplement, World};

fn qser<T: serde::Serialize>(t: PathBuf, obj: &T) -> std::io::Result<()> {
    let out = DeflateEncoder::new(BufWriter::new(File::create(t)?), Compression::default());
    bincode::serialize_into(out, obj).unwrap();
    Ok(())
}

fn qdeser<T: serde::de::DeserializeOwned>(t: PathBuf) -> std::io::Result<T> {
    let r = DeflateDecoder::new(BufReader::new(File::open(t)?));
    let val = bincode::deserialize_from(r).unwrap();
    Ok(val)
}

pub enum SaveMsg {
    END,
    SAVE(Vec2<i32>, TerrainChunk),
    //RATE(u64),
}

pub struct Provider {
    pub world: World,
    pub target: PathBuf,
    save_handle: Option<std::thread::JoinHandle<()>>,

    pub tx: Option<channel::Sender<SaveMsg>>,
}

impl Provider {
    pub fn new(seed: u32, target: PathBuf) -> Self {
        let world = Self::load(target.clone()).unwrap_or_else(|_| {
            /*if target.exists() {
                println!("Failed to open {:?}/, moving to {:?}.old/", target, target);
                std::fs::rename(target.clone(), target.clone().with_extension("old"))
                    .unwrap_or_else(|_| println!("Ok, something strange is happening here..."));
            } else {*/
            std::fs::create_dir_all(target.clone()).unwrap();
            //}
            World::generate(seed)
        });

        let mut this = Self {
            world,
            target,
            save_handle: None,
            tx: None,
        };

        this.init_save_loop();

        this
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

    pub fn init_save_loop(&mut self) {
        let (tx, rx) = channel::unbounded::<SaveMsg>();
        self.tx = Some(tx);

        let tgt = self.target.clone();
        let t = move |v: Vec2<i32>| tgt.join(Self::chunk_name(v));
        self.save_handle = Some(thread::spawn(move || 'yeet: loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    SaveMsg::END => {
                        log::info!("Wrapping up world...");
                        break 'yeet;
                    }
                    SaveMsg::SAVE(pos, chunk) => {
                        qser(t(pos), &chunk).unwrap_or_else(|err| {
                            log::warn!("Failed to save chunk {}: {:?}", pos, err)
                        });
                    }
                }
            }
        }));
    }

    pub fn set_chunk(&self, pos: Vec2<i32>, chunk: TerrainChunk) {
        self.request_save_message(SaveMsg::SAVE(pos, chunk));
    }

    pub fn request_save_message(&self, msg: SaveMsg) {
        if let Some(tx) = &self.tx {
            tx.send(msg).unwrap();
        }
    }

    pub fn load(target: PathBuf) -> std::io::Result<World> {
        let t = |val: &str| target.join(val);
        let chunks = qdeser(t("chunks"))?;
        let locations = qdeser(t("locations"))?;
        let seed = qdeser(t("seed"))?;
        let (gen_ctx, rng) = sim::GenCtx::from_seed(seed);

        Ok(World {
            sim: sim::WorldSim {
                chunks,
                locations,
                seed,
                gen_ctx,
                rng,
            },
        })
    }

    pub fn fetch_chunk(
        &self,
        chunk_pos: Vec2<i32>,
        cancel: Arc<AtomicBool>,
    ) -> Result<(TerrainChunk, ChunkSupplement), ()> {
        match qdeser(self.chunk_path(chunk_pos)) {
            Ok(chunk) => Ok((chunk, ChunkSupplement::default())),
            Err(_) => self
                .world
                .generate_chunk(chunk_pos, || cancel.load(Ordering::Relaxed)),
        }
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        self.request_save_message(SaveMsg::END);
        if let Some(handle) = self.save_handle.take() {
            handle.join().unwrap();
        }
    }
}
