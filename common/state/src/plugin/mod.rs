pub mod errors;
pub mod memory_manager;
pub mod module;

use bincode::ErrorKind;
use common::{assets::ASSETS_PATH, uid::Uid};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use tracing::{error, info};

use self::{
    errors::{PluginError, PluginModuleError},
    memory_manager::EcsWorld,
    module::PluginModule,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginData {
    name: String,
    modules: HashSet<PathBuf>,
    dependencies: HashSet<String>,
}

pub struct Plugin {
    data: PluginData,
    modules: Vec<PluginModule>,
    #[allow(dead_code)]
    files: HashMap<PathBuf, Vec<u8>>,
}

impl Plugin {
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, PluginError> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(PluginError::Io)?;

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

        Ok(Plugin {
            data,
            modules,
            files,
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
}

#[derive(Default)]
pub struct PluginMgr {
    plugins: Vec<Plugin>,
}

impl PluginMgr {
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
                    Plugin::from_reader(fs::File::open(entry.path()).map_err(PluginError::Io)?).map(
                        |plugin| {
                            if let Err(e) = common::assets::register_tar(entry.path()) {
                                error!("Plugin {:?} tar error {e:?}", entry.path());
                            }
                            Some(plugin)
                        },
                    )
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
