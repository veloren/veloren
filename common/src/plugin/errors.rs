use bincode::ErrorKind;
use wasmer_runtime::error::{ResolveError, RuntimeError};

#[derive(Debug)]
pub enum PluginError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    NoConfig,
    NoSuchModule,
    PluginModuleError(PluginModuleError)
}

#[derive(Debug)]
pub enum PluginModuleError {
    FindFunction(String),
    FunctionGet(ResolveError),
    Compile(wasmer_runtime::error::CompileError),
    Instantiate(wasmer_runtime::error::Error),
    RunFunction(RuntimeError),
    Encoding(Box<ErrorKind>),
}