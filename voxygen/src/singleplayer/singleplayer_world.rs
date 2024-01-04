use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use common::{assets::ASSETS_PATH, consts::DAY_LENGTH_DEFAULT};
use serde::{Deserialize, Serialize};
use server::{FileOpts, GenOpts, DEFAULT_WORLD_MAP};
use tracing::error;

#[derive(Clone, Deserialize, Serialize)]
struct World0 {
    name: String,
    gen_opts: Option<GenOpts>,
    seed: u32,
}

pub struct SingleplayerWorld {
    pub name: String,
    pub gen_opts: Option<GenOpts>,
    pub day_length: f64,
    pub seed: u32,
    pub is_generated: bool,
    pub path: PathBuf,
    pub map_path: PathBuf,
}

impl SingleplayerWorld {
    pub fn copy_default_world(&self) {
        if let Err(e) = fs::copy(asset_path(DEFAULT_WORLD_MAP), &self.map_path) {
            println!("Error when trying to copy default world: {e}");
        }
    }
}

fn load_map(path: &Path) -> Option<SingleplayerWorld> {
    let meta_path = path.join("meta.ron");

    let Ok(f) = fs::File::open(&meta_path) else {
        error!("Failed to open {}", meta_path.to_string_lossy());
        return None;
    };

    let Ok(bytes) = f.bytes().collect::<Result<Vec<u8>, _>>() else {
        error!("Failed to read {}", meta_path.to_string_lossy());
        return None;
    };

    version::try_load(std::io::Cursor::new(bytes), path)
}

fn write_world_meta(world: &SingleplayerWorld) {
    let path = &world.path;

    if let Err(e) = fs::create_dir_all(path) {
        error!("Failed to create world folder: {e}");
    }

    match fs::File::create(path.join("meta.ron")) {
        Ok(file) => {
            if let Err(e) = ron::ser::to_writer_pretty(
                file,
                &version::Current::from_world(world),
                ron::ser::PrettyConfig::new(),
            ) {
                error!("Failed to create world meta file: {e}")
            }
        },
        Err(e) => error!("Failed to create world meta file: {e}"),
    }
}

fn asset_path(asset: &str) -> PathBuf {
    let mut s = asset.replace('.', "/");
    s.push_str(".bin");
    ASSETS_PATH.join(s)
}

fn migrate_old_singleplayer(from: &Path, to: &Path) {
    if fs::metadata(from).map_or(false, |meta| meta.is_dir()) {
        if let Err(e) = fs::rename(from, to) {
            error!("Failed to migrate singleplayer: {e}");
            return;
        }

        let mut seed = 0;
        let mut day_length = DAY_LENGTH_DEFAULT;
        let (map_file, gen_opts) = fs::read_to_string(to.join("server_config/settings.ron"))
            .ok()
            .and_then(|settings| {
                let settings: server::Settings = ron::from_str(&settings).ok()?;
                seed = settings.world_seed;
                day_length = settings.day_length;
                Some(match settings.map_file? {
                    FileOpts::LoadOrGenerate { name, opts, .. } => {
                        (Some(PathBuf::from(name)), Some(opts))
                    },
                    FileOpts::Generate(opts) => (None, Some(opts)),
                    FileOpts::LoadLegacy(_) => return None,
                    FileOpts::Load(path) => (Some(path), None),
                    FileOpts::LoadAsset(asset) => (Some(asset_path(&asset)), None),
                    FileOpts::Save(_, gen_opts) => (None, Some(gen_opts)),
                })
            })
            .unwrap_or((Some(asset_path(DEFAULT_WORLD_MAP)), None));

        let map_path = to.join("map.bin");
        if let Some(map_file) = map_file {
            if let Err(err) = fs::copy(map_file, &map_path) {
                error!("Failed to copy map file to singleplayer world: {err}");
            }
        }

        write_world_meta(&SingleplayerWorld {
            name: "singleplayer world".to_string(),
            gen_opts,
            seed,
            day_length,
            path: to.to_path_buf(),
            // Isn't persisted so doesn't matter what it's set to.
            is_generated: false,
            map_path,
        });
    }
}

fn load_worlds(path: &Path) -> Vec<SingleplayerWorld> {
    let Ok(paths) = fs::read_dir(path) else {
        let _ = fs::create_dir_all(path);
        return Vec::new();
    };

    paths
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                let path = entry.path();
                load_map(&path)
            } else {
                None
            }
        })
        .collect()
}

#[derive(Default)]
pub struct SingleplayerWorlds {
    pub worlds: Vec<SingleplayerWorld>,
    pub current: Option<usize>,
    worlds_folder: PathBuf,
}

impl SingleplayerWorlds {
    pub fn load(userdata_folder: &Path) -> SingleplayerWorlds {
        let worlds_folder = userdata_folder.join("singleplayer_worlds");

        if let Err(e) = fs::create_dir_all(&worlds_folder) {
            error!("Failed to create singleplayer worlds folder: {e}");
        }

        migrate_old_singleplayer(
            &userdata_folder.join("singleplayer"),
            &worlds_folder.join("singleplayer"),
        );

        let worlds = load_worlds(&worlds_folder);

        SingleplayerWorlds {
            worlds,
            current: None,
            worlds_folder,
        }
    }

    pub fn delete_map_file(&mut self, map: usize) {
        let w = &mut self.worlds[map];
        if w.is_generated {
            // We don't care about the result here since we aren't sure the file exists.
            let _ = fs::remove_file(&w.map_path);
        }
        w.is_generated = false;
    }

    pub fn remove(&mut self, idx: usize) {
        if let Some(ref mut i) = self.current {
            match (*i).cmp(&idx) {
                std::cmp::Ordering::Less => {},
                std::cmp::Ordering::Equal => self.current = None,
                std::cmp::Ordering::Greater => *i -= 1,
            }
        }
        let _ = fs::remove_dir_all(&self.worlds[idx].path);
        self.worlds.remove(idx);
    }

    fn world_folder_name(&self) -> String {
        use chrono::{Datelike, Timelike};
        let now = chrono::Local::now().naive_local();
        let name = format!(
            "world-{}-{}-{}-{}_{}_{}_{}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
            now.timestamp_subsec_millis()
        );

        let mut test_name = name.clone();
        let mut i = 0;
        'fail: loop {
            for world in self.worlds.iter() {
                if world.path.ends_with(&test_name) {
                    test_name = name.clone();
                    test_name.push('_');
                    test_name.push_str(&i.to_string());
                    i += 1;
                    continue 'fail;
                }
            }
            break;
        }
        test_name
    }

    pub fn current(&self) -> Option<&SingleplayerWorld> {
        self.current.and_then(|i| self.worlds.get(i))
    }

    pub fn new_world(&mut self) {
        let folder_name = self.world_folder_name();
        let path = self.worlds_folder.join(folder_name);

        let new_world = SingleplayerWorld {
            name: "New World".to_string(),
            gen_opts: None,
            day_length: DAY_LENGTH_DEFAULT,
            seed: 0,
            is_generated: false,
            map_path: path.join("map.bin"),
            path,
        };

        write_world_meta(&new_world);

        self.worlds.push(new_world)
    }

    pub fn save_current_meta(&self) {
        if let Some(world) = self.current() {
            write_world_meta(world);
        }
    }
}

mod version {
    use std::any::{type_name, Any};

    use serde::de::DeserializeOwned;

    use super::*;

    pub type Current = V2;

    type LoadWorldFn<R> =
        fn(R, &Path) -> Result<SingleplayerWorld, (&'static str, ron::de::SpannedError)>;
    fn loaders<'a, R: std::io::Read + Clone>() -> &'a [LoadWorldFn<R>] {
        // Step [4]
        &[load_raw::<V2, _>, load_raw::<V1, _>]
    }

    #[derive(Deserialize, Serialize)]
    pub struct V1 {
        #[serde(deserialize_with = "version::<_, 1>")]
        version: u64,
        name: String,
        gen_opts: Option<GenOpts>,
        seed: u32,
    }

    impl ToWorld for V1 {
        fn to_world(self, path: PathBuf) -> SingleplayerWorld {
            let map_path = path.join("map.bin");
            let is_generated = fs::metadata(&map_path).is_ok_and(|f| f.is_file());

            SingleplayerWorld {
                name: self.name,
                gen_opts: self.gen_opts,
                seed: self.seed,
                day_length: DAY_LENGTH_DEFAULT,
                is_generated,
                path,
                map_path,
            }
        }
    }

    #[derive(Deserialize, Serialize)]
    pub struct V2 {
        #[serde(deserialize_with = "version::<_, 2>")]
        version: u64,
        name: String,
        gen_opts: Option<GenOpts>,
        seed: u32,
        day_length: f64,
    }

    impl V2 {
        pub fn from_world(world: &SingleplayerWorld) -> Self {
            V2 {
                version: 2,
                name: world.name.clone(),
                gen_opts: world.gen_opts.clone(),
                seed: world.seed,
                day_length: world.day_length,
            }
        }
    }

    impl ToWorld for V2 {
        fn to_world(self, path: PathBuf) -> SingleplayerWorld {
            let map_path = path.join("map.bin");
            let is_generated = fs::metadata(&map_path).is_ok_and(|f| f.is_file());

            SingleplayerWorld {
                name: self.name,
                gen_opts: self.gen_opts,
                seed: self.seed,
                day_length: self.day_length,
                is_generated,
                path,
                map_path,
            }
        }
    }

    // Utilities
    fn version<'de, D: serde::Deserializer<'de>, const V: u64>(de: D) -> Result<u64, D::Error> {
        u64::deserialize(de).and_then(|x| {
            if x == V {
                Ok(x)
            } else {
                Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Unsigned(x),
                    &"incorrect magic/version bytes",
                ))
            }
        })
    }

    trait ToWorld {
        fn to_world(self, path: PathBuf) -> SingleplayerWorld;
    }

    fn load_raw<RawWorld: Any + ToWorld + DeserializeOwned, R: std::io::Read + Clone>(
        reader: R,
        path: &Path,
    ) -> Result<SingleplayerWorld, (&'static str, ron::de::SpannedError)> {
        ron::de::from_reader::<_, RawWorld>(reader)
            .map(|s| s.to_world(path.to_path_buf()))
            .map_err(|e| (type_name::<RawWorld>(), e))
    }

    pub fn try_load<R: std::io::Read + Clone>(reader: R, path: &Path) -> Option<SingleplayerWorld> {
        loaders()
            .iter()
            .find_map(|load_raw| match load_raw(reader.clone(), path) {
                Ok(chunk) => Some(chunk),
                Err((raw_name, e)) => {
                    error!(
                        "Attempt to load chunk with raw format `{}` failed: {:?}",
                        raw_name, e
                    );
                    None
                },
            })
    }
}
