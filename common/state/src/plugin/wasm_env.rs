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
    pub ecs: Arc<EcsAccessManager>, /* This represent the pointer to the ECS object (set to
                                     * i32::MAX if to ECS is
                                     * availible) */
    pub memory: Option<Memory>, // This object represent the WASM Memory
    pub allocator: Option<
        TypedFunction<<MemoryModel as wasmer::MemorySize>::Offset, WasmPtr<u8, MemoryModel>>,
    >, /* Linked to: wasm_prepare_buffer */
    pub name: String,           // This represent the plugin name
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
    pub fn new(name: String, ecs: Arc<EcsAccessManager>) -> Self {
        Self {
            ecs,
            allocator: Default::default(),
            memory: Default::default(),
            name,
        }
    }

    #[inline]
    pub fn ecs(&self) -> &Arc<EcsAccessManager> { &self.ecs }

    #[inline]
    pub fn memory(&self) -> &Memory { self.memory.as_ref().unwrap() }

    #[inline]
    pub fn allocator(
        &self,
    ) -> &TypedFunction<<MemoryModel as wasmer::MemorySize>::Offset, WasmPtr<u8, MemoryModel>> {
        self.allocator.as_ref().unwrap()
    }

    #[inline]
    pub fn name(&self) -> &str { &self.name }

    /// This function is a safe interface to WASM memory that writes data to the
    /// memory returning a pointer and length
    pub fn write_data<T: Serialize>(
        &self,
        store: &mut StoreMut,
        object: &T,
    ) -> Result<
        (
            WasmPtr<u8, MemoryModel>,
            <MemoryModel as wasmer::MemorySize>::Offset,
        ),
        PluginModuleError,
    > {
        memory_manager::write_data(store, self.memory(), self.allocator(), object)
    }

    /// This function is a safe interface to WASM memory that writes data to the
    /// memory returning a pointer and length
    pub fn write_data_as_pointer<T: Serialize>(
        &self,
        store: &mut StoreMut,
        object: &T,
    ) -> Result<WasmPtr<u8, MemoryModel>, PluginModuleError> {
        memory_manager::write_data_as_pointer(store, self.memory(), self.allocator(), object)
    }

    /// This function is a safe interface to WASM memory that reads memory from
    /// pointer and length returning an object
    pub fn read_data<T: DeserializeOwned>(
        &self,
        store: &StoreRef,
        position: WasmPtr<u8, MemoryModel>,
        length: <MemoryModel as wasmer::MemorySize>::Offset,
    ) -> Result<T, bincode::Error> {
        memory_manager::read_data(self.memory(), store, position, length)
    }

    /// This function is a safe interface to WASM memory that reads memory from
    /// a pointer and a length and returns some bytes
    pub fn read_bytes(
        &self,
        store: &StoreRef,
        ptr: WasmPtr<u8, MemoryModel>,
        len: <MemoryModel as wasmer::MemorySize>::Offset,
    ) -> Result<Vec<u8>, PluginModuleError> {
        self.memory.as_ref().map_or_else(
            || Err(PluginModuleError::InvalidPointer),
            |m| memory_manager::read_bytes(m, store, ptr, len),
        )
    }

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

    pub fn init_with_instance(&mut self, args: HostFunctionEnvironmentInit) {
        self.memory = Some(args.memory);
        self.allocator = Some(args.allocator);
    }
}
