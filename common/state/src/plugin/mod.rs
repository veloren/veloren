pub mod errors;
pub mod exports;
pub mod memory_manager;
pub mod module;
pub mod wasm_env;

use bincode::ErrorKind;
use common::assets::ASSETS_PATH;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use tracing::{error, info};
use wasmer::Memory64;

use plugin_api::Event;

use self::{
    errors::PluginError,
    memory_manager::EcsWorld,
    module::{PluginModule, PreparedEventQuery},
    wasm_env::HostFunctionException,
};

use rayon::prelude::*;

pub type MemoryModel = Memory64;

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

    pub fn execute_prepared<T>(
        &mut self,
        ecs: &EcsWorld,
        event: &PreparedEventQuery<T>,
    ) -> Result<Vec<T::Response>, PluginError>
    where
        T: Event,
    {
        self.modules
            .iter_mut()
            .flat_map(|module| {
                module.try_execute(ecs, event).map(|x| {
                    x.map_err(|e| {
                        if let errors::PluginModuleError::RunFunction(runtime_err) = &e {
                            if let Some(host_except) =
                                runtime_err.downcast_ref::<HostFunctionException>()
                            {
                                match host_except {
                                    HostFunctionException::ProcessExit(code) => {
                                        module.exit_code = Some(*code);
                                        tracing::warn!(
                                            "Module {} binary {} exited with {}",
                                            self.data.name,
                                            module.name(),
                                            *code
                                        );
                                        return PluginError::ProcessExit;
                                    },
                                }
                            }
                        }
                        PluginError::PluginModuleError(
                            self.data.name.to_owned(),
                            event.get_function_name().to_owned(),
                            e,
                        )
                    })
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                if matches!(e, PluginError::ProcessExit) {
                    // remove the executable from the module which called process exit
                    self.modules.retain(|m| m.exit_code.is_none())
                }
                e
            })
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

    pub fn execute_prepared<T>(
        &mut self,
        ecs: &EcsWorld,
        event: &PreparedEventQuery<T>,
    ) -> Result<Vec<T::Response>, PluginError>
    where
        T: Event,
    {
        Ok(self
            .plugins
            .par_iter_mut()
            .map(|plugin| plugin.execute_prepared(ecs, event))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect())
    }

    pub fn execute_event<T>(
        &mut self,
        ecs: &EcsWorld,
        event: &T,
    ) -> Result<Vec<T::Response>, PluginError>
    where
        T: Event,
    {
        self.execute_prepared(ecs, &PreparedEventQuery::new(event)?)
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
                        |o| {
                            if let Err(e) = common::assets::register_tar(entry.path()) {
                                error!("Plugin {:?} tar error {e:?}", entry.path());
                            }
                            Some(o)
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
}
