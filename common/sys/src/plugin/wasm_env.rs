use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use wasmer::{Function, HostEnvInitError, Instance, LazyInit, Memory, WasmerEnv};

use super::{
    errors::PluginModuleError,
    memory_manager::{self, EcsAccessManager, MemoryManager},
};

#[derive(Clone)]
pub struct HostFunctionEnvironement {
    pub ecs: Arc<EcsAccessManager>, /* This represent the pointer to the ECS object (set to
                                     * i32::MAX if to ECS is
                                     * availible) */
    pub memory: LazyInit<Memory>, // This object represent the WASM Memory
    pub allocator: LazyInit<Function>, // Linked to: wasm_prepare_buffer
    pub memory_manager: Arc<MemoryManager>, /* This object represent the current buffer size and
                                   * pointer */
    pub name: String, // This represent the plugin name
}

impl HostFunctionEnvironement {
    pub fn new(
        name: String,
        ecs: Arc<EcsAccessManager>,
        memory_manager: Arc<MemoryManager>,
    ) -> Self {
        Self {
            memory_manager,
            ecs,
            allocator: LazyInit::new(),
            memory: LazyInit::new(),
            name,
        }
    }

    /// This function is a safe interface to WASM memory that writes data to the
    /// memory returning a pointer and length
    pub fn write_data<T: Serialize>(&self, object: &T) -> Result<(u64, u64), PluginModuleError> {
        self.memory_manager.write_data(
            self.memory.get_ref().unwrap(),
            self.allocator.get_ref().unwrap(),
            object,
        )
    }

    /// This function is a safe interface to WASM memory that writes data to the
    /// memory returning a pointer and length
    pub fn write_data_as_pointer<T: Serialize>(
        &self,
        object: &T,
    ) -> Result<u64, PluginModuleError> {
        self.memory_manager.write_data_as_pointer(
            self.memory.get_ref().unwrap(),
            self.allocator.get_ref().unwrap(),
            object,
        )
    }

    /// This function is a safe interface to WASM memory that reads memory from
    /// pointer and length returning an object
    pub fn read_data<T: DeserializeOwned>(
        &self,
        position: u64,
        length: u64,
    ) -> Result<T, bincode::Error> {
        memory_manager::read_data(self.memory.get_ref().unwrap(), position, length)
    }
}

impl WasmerEnv for HostFunctionEnvironement {
    fn init_with_instance(&mut self, instance: &Instance) -> Result<(), HostEnvInitError> {
        let memory = instance.exports.get_memory("memory").unwrap();
        self.memory.initialize(memory.clone());
        let allocator = instance
            .exports
            .get_function("wasm_prepare_buffer")
            .expect("Can't get allocator");
        self.allocator.initialize(allocator.clone());
        Ok(())
    }
}
