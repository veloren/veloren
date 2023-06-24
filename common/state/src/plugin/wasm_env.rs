use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use wasmer::{ExportError, Instance, Memory, Store, StoreMut, StoreRef, TypedFunction, WasmPtr};

use super::{
    errors::PluginModuleError,
    memory_manager::{self, EcsAccessManager},
    MemoryModel,
};

#[derive(Clone)]
pub struct HostFunctionEnvironment {
    ecs: Arc<EcsAccessManager>, /* This represent the pointer to the ECS object (set to
                                 * i32::MAX if to ECS is
                                 * availible) */
    memory: Option<Memory>, // This object represent the WASM Memory
    allocator: Option<
        TypedFunction<<MemoryModel as wasmer::MemorySize>::Offset, WasmPtr<u8, MemoryModel>>,
    >, /* Linked to: wasm_prepare_buffer */
    name: String,           // This represent the plugin name
}

pub struct HostFunctionEnvironmentInit {
    allocator: TypedFunction<<MemoryModel as wasmer::MemorySize>::Offset, WasmPtr<u8, MemoryModel>>,
    memory: Memory,
}

#[derive(Debug, Clone, Copy)]
// Exception thrown from a native wasm callback
pub enum HostFunctionException {
    ProcessExit(i32),
}

// needed for `std::error::Error`
impl core::fmt::Display for HostFunctionException {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { write!(f, "{:?}", self) }
}

impl std::error::Error for HostFunctionException {}

impl HostFunctionEnvironment {
    /// Create a new environment for functions providing functionality to WASM
    pub fn new(name: String, ecs: Arc<EcsAccessManager>) -> Self {
        Self {
            ecs,
            allocator: Default::default(),
            memory: Default::default(),
            name,
        }
    }

    #[inline]
    pub(crate) fn ecs(&self) -> &Arc<EcsAccessManager> { &self.ecs }

    #[inline]
    pub(crate) fn memory(&self) -> &Memory { self.memory.as_ref().unwrap() }

    #[inline]
    pub(crate) fn allocator(
        &self,
    ) -> &TypedFunction<<MemoryModel as wasmer::MemorySize>::Offset, WasmPtr<u8, MemoryModel>> {
        self.allocator.as_ref().unwrap()
    }

    #[inline]
    pub(crate) fn name(&self) -> &str { &self.name }

    /// This function is a safe interface to WASM memory that serializes and
    /// writes an object to linear memory returning a pointer
    pub(crate) fn write_serialized_with_length<T: Serialize>(
        &self,
        store: &mut StoreMut,
        object: &T,
    ) -> Result<WasmPtr<u8, MemoryModel>, PluginModuleError> {
        memory_manager::write_serialized_with_length(store, self.memory(), self.allocator(), object)
    }

    /// This function is a safe interface to WASM memory that reads memory from
    /// pointer and length returning an object
    pub(crate) fn read_serialized<T: DeserializeOwned>(
        &self,
        store: &StoreRef,
        position: WasmPtr<u8, MemoryModel>,
        length: <MemoryModel as wasmer::MemorySize>::Offset,
    ) -> Result<T, bincode::Error> {
        memory_manager::read_serialized(self.memory(), store, position, length)
    }

    /// This function is a safe interface to WASM memory that reads memory from
    /// a pointer and a length and returns some bytes
    pub(crate) fn read_bytes(
        &self,
        store: &StoreRef,
        ptr: WasmPtr<u8, MemoryModel>,
        len: <MemoryModel as wasmer::MemorySize>::Offset,
    ) -> Result<Vec<u8>, PluginModuleError> {
        memory_manager::read_bytes(self.memory(), store, ptr, len)
    }

    /// This function creates the argument for init_with_instance() from
    /// exported symbol lookup
    pub fn args_from_instance(
        store: &Store,
        instance: &Instance,
    ) -> Result<HostFunctionEnvironmentInit, ExportError> {
        let memory = instance.exports.get_memory("memory")?.clone();
        let allocator = instance
            .exports
            .get_typed_function(store, "wasm_prepare_buffer")?;
        Ok(HostFunctionEnvironmentInit { memory, allocator })
    }

    /// Initialize the wasm exports in the environment
    pub fn init_with_instance(&mut self, args: HostFunctionEnvironmentInit) {
        self.memory = Some(args.memory);
        self.allocator = Some(args.allocator);
    }
}
