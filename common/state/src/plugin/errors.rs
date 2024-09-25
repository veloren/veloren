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
