pub mod errors;
pub mod memory_manager;
pub mod module;

use bincode::ErrorKind;
use common::{assets::ASSETS_PATH, event::PluginHash, uid::Uid};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tracing::{error, info};

use self::{
    errors::{PluginError, PluginModuleError},
    memory_manager::EcsWorld,
    module::PluginModule,
};

use sha2::Digest;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginData {
    name: String,
    modules: HashSet<PathBuf>,
    dependencies: HashSet<String>,
}

fn compute_hash(data: &[u8]) -> PluginHash {
    let shasum = sha2::Sha256::digest(data);
    let mut shasum_iter = shasum.iter();
    // a newer generic-array supports into_array ...
    let shasum: PluginHash = std::array::from_fn(|_| *shasum_iter.next().unwrap());
    shasum
}

fn cache_file_name(
    mut base_dir: PathBuf,
    hash: &PluginHash,
    create_dir: bool,
) -> Result<PathBuf, std::io::Error> {
    base_dir.push("server-plugins");
    if create_dir {
        std::fs::create_dir_all(base_dir.as_path())?;
    }
    let name = hex::encode(hash);
    base_dir.push(name);
    base_dir.set_extension("plugin.tar");
    Ok(base_dir)
}

// write received plugin to disk cache
pub fn store_server_plugin(base_dir: &Path, data: Vec<u8>) -> Result<PathBuf, std::io::Error> {
    let shasum = compute_hash(data.as_slice());
    let result = cache_file_name(base_dir.to_path_buf(), &shasum, true)?;
    let mut file = std::fs::File::create(result.as_path())?;
    file.write_all(data.as_slice())?;
    Ok(result)
}

pub fn find_cached(base_dir: &Path, hash: &PluginHash) -> Result<PathBuf, std::io::Error> {
    let local_path = cache_file_name(base_dir.to_path_buf(), hash, false)?;
    if local_path.as_path().exists() {
        Ok(local_path)
    } else {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }
}

pub struct Plugin {
    data: PluginData,
    modules: Vec<PluginModule>,
    #[allow(dead_code)]
    hash: PluginHash,
    #[allow(dead_code)]
    path: PathBuf,
    #[allow(dead_code)]
    data_buf: Vec<u8>,
}

impl Plugin {
    pub fn from_path(path_buf: PathBuf) -> Result<Self, PluginError> {
        let mut reader = fs::File::open(path_buf.as_path()).map_err(PluginError::Io)?;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(PluginError::Io)?;
        let shasum = compute_hash(buf.as_slice());

        let mut files = tar::Archive::new(&*buf)
            .entries()
            .map_err(PluginError::Io)?
            .map(|e| {
                e.and_then(|e| {
                    Ok((e.path()?.into_owned(), {
                        let offset = e.raw_file_position() as usize;
                        buf[offset..offset + e.size() as usize].to_vec()
                    }))
                })
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .map_err(PluginError::Io)?;

        let data = toml::de::from_str::<PluginData>(
            std::str::from_utf8(
                files
                    .get(Path::new("plugin.toml"))
                    .ok_or(PluginError::NoConfig)?,
            )
            .map_err(|e| PluginError::Encoding(Box::new(ErrorKind::InvalidUtf8Encoding(e))))?,
        )
        .map_err(PluginError::Toml)?;

        let modules = data
            .modules
            .iter()
            .map(|path| {
                let wasm_data = files.remove(path).ok_or(PluginError::NoSuchModule)?;
                PluginModule::new(data.name.to_owned(), &wasm_data).map_err(|e| {
                    PluginError::PluginModuleError(data.name.to_owned(), "<init>".to_owned(), e)
                })
            })
            .collect::<Result<_, _>>()?;

        let data_buf = fs::read(&path_buf).map_err(PluginError::Io)?;

        Ok(Plugin {
            data,
            modules,
            hash: shasum,
            path: path_buf,
            data_buf,
        })
    }

    pub fn load_event(
        &mut self,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<(), PluginModuleError> {
        self.modules
            .iter_mut()
            .try_for_each(|module| module.load_event(ecs, mode))
    }

    pub fn command_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        args: &[String],
        player: common::uid::Uid,
    ) -> Result<Vec<String>, CommandResults> {
        let mut result = Err(CommandResults::UnknownCommand);
        self.modules.iter_mut().for_each(|module| {
            match module.command_event(ecs, name, args, player) {
                Ok(res) => result = Ok(res),
                Err(CommandResults::UnknownCommand) => (),
                Err(err) => {
                    if result.is_err() {
                        result = Err(err)
                    }
                },
            }
        });
        result
    }

    /// get the path to the plugin file
    pub fn path(&self) -> &Path { self.path.as_path() }

    /// Get the data of this plugin
    pub fn data_buf(&self) -> &[u8] { &self.data_buf }
}

#[derive(Default)]
pub struct PluginMgr {
    plugins: Vec<Plugin>,
}

impl PluginMgr {
    pub fn from_asset_or_default() -> Self {
        match Self::from_assets() {
            Ok(plugin_mgr) => plugin_mgr,
            Err(e) => {
                tracing::error!(?e, "Failed to read plugins from assets");
                PluginMgr::default()
            },
        }
    }

    pub fn from_assets() -> Result<Self, PluginError> {
        let mut assets_path = (*ASSETS_PATH).clone();
        assets_path.push("plugins");
        info!("Searching {:?} for plugins...", assets_path);
        Self::from_dir(assets_path)
    }

    pub fn from_dir<P: AsRef<Path>>(path: P) -> Result<Self, PluginError> {
        let plugins = fs::read_dir(path)
            .map_err(PluginError::Io)?
            .filter_map(|e| e.ok())
            .map(|entry| {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                    && entry
                        .path()
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.ends_with(".plugin.tar"))
                        .unwrap_or(false)
                {
                    info!("Loading plugin at {:?}", entry.path());
                    Plugin::from_path(entry.path()).map(|plugin| {
                        if let Err(e) = common::assets::register_tar(entry.path()) {
                            error!("Plugin {:?} tar error {e:?}", entry.path());
                        }
                        Some(plugin)
                    })
                } else {
                    Ok(None)
                }
            })
            .filter_map(Result::transpose)
            .inspect(|p| {
                let _ = p.as_ref().map_err(|e| error!(?e, "Failed to load plugin"));
            })
            .collect::<Result<Vec<_>, _>>()?;

        for plugin in &plugins {
            info!(
                "Loaded plugin '{}' with {} module(s)",
                plugin.data.name,
                plugin.modules.len()
            );
        }

        Ok(Self { plugins })
    }

    /// Add a plugin received from the server
    pub fn load_server_plugin(&mut self, path: PathBuf) -> Result<PluginHash, PluginError> {
        Plugin::from_path(path.clone()).map(|plugin| {
            if let Err(e) = common::assets::register_tar(path.clone()) {
                error!("Plugin {:?} tar error {e:?}", path.as_path());
            }
            let hash = plugin.hash;
            self.plugins.push(plugin);
            hash
        })
    }

    pub fn cache_server_plugin(
        &mut self,
        base_dir: &Path,
        data: Vec<u8>,
    ) -> Result<PluginHash, PluginError> {
        let path = store_server_plugin(base_dir, data).map_err(PluginError::Io)?;
        self.load_server_plugin(path)
    }

    /// list all registered plugins
    pub fn plugin_list(&self) -> Vec<PluginHash> {
        self.plugins.iter().map(|plugin| plugin.hash).collect()
    }

    /// retrieve a specific plugin
    pub fn find(&self, hash: &PluginHash) -> Option<&Plugin> {
        self.plugins.iter().find(|plugin| &plugin.hash == hash)
    }

    pub fn load_event(
        &mut self,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<(), PluginModuleError> {
        self.plugins
            .iter_mut()
            .try_for_each(|plugin| plugin.load_event(ecs, mode))
    }

    pub fn command_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        args: &[String],
        player: Uid,
    ) -> Result<Vec<String>, CommandResults> {
        // return last value or last error
        let mut result = Err(CommandResults::UnknownCommand);
        self.plugins.iter_mut().for_each(|plugin| {
            match plugin.command_event(ecs, name, args, player) {
                Ok(val) => result = Ok(val),
                Err(CommandResults::UnknownCommand) => (),
                Err(err) => {
                    if result.is_err() {
                        result = Err(err);
                    }
                },
            }
        });
        result
    }
}

/// Error returned by plugin based server commands
pub enum CommandResults {
    UnknownCommand,
    HostError(wasmtime::Error),
    PluginError(String),
}
