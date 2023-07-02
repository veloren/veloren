use bincode::ErrorKind;
use wasmer::{CompileError, ExportError, InstantiationError, RuntimeError};

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
    InstantiationError(Box<InstantiationError>),
    InvalidPointer,
    MemoryAllocation(MemoryAllocationError),
    MemoryUninit(ExportError),
    FindFunction(ExportError),
    RunFunction(RuntimeError),
    InvalidArgumentType(),
    Encoding(Box<ErrorKind>),
    CompileError(CompileError),
}

#[derive(Debug)]
pub enum MemoryAllocationError {
    InvalidReturnType,
    AllocatorNotFound(ExportError),
    CantAllocate(RuntimeError),
}
