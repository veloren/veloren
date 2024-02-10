use bincode::ErrorKind;

#[derive(Debug)]
pub enum PluginError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    NoConfig,
    NoSuchModule,
    Encoding(Box<ErrorKind>),
    PluginModuleError(String, String, PluginModuleError),
    ProcessExit,
}

#[derive(Debug)]
pub enum PluginModuleError {
    Wasmtime(wasmtime::Error),
}

#[derive(Debug)]
pub enum EcsAccessError {
    EcsPointerNotAvailable,
    EcsComponentNotFound(common::uid::Uid, String),
    EcsResourceNotFound(String),
    EcsEntityNotFound(common::uid::Uid),
}

impl std::fmt::Display for EcsAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for EcsAccessError {}
