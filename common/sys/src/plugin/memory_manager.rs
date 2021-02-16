use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicU32, AtomicU64, Ordering};

use serde::{de::DeserializeOwned, Serialize};
use specs::World;
use wasmer::{Function, Memory, Value};

use super::errors::{MemoryAllocationError, PluginModuleError};

// This structure wraps the ECS pointer to ensure safety
pub struct EcsAccessManager {
    ecs_pointer: AtomicPtr<World>,
}

impl Default for EcsAccessManager {

    fn default() -> Self {
        Self {
            ecs_pointer: AtomicPtr::new(std::ptr::null_mut::<_>())
        }
    }
}

impl EcsAccessManager {

    pub fn execute_with<T>(&self, world: &World, func: impl FnOnce() -> T) -> T {
        self.ecs_pointer.store(world as *const _ as *mut _, Ordering::SeqCst);
        let out = func();
        self.ecs_pointer.store(std::ptr::null_mut::<_>(), Ordering::SeqCst);
        out
    }

    pub fn get(&self) -> Option<&World> {
        // ptr::as_ref will automatically check for null
        unsafe {self.ecs_pointer.load(Ordering::SeqCst).as_ref()}
    }
}

pub struct MemoryManager {
    pub pointer: AtomicI32,
    pub length: AtomicU32,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            pointer: AtomicI32::new(0),
            length: AtomicU32::new(0),
        }
    }
}

impl MemoryManager {

    // This function check if the buffer is wide enough if not it realloc the buffer
    // calling the `wasm_prepare_buffer` function Note: There is probably
    // optimizations that can be done using less restrictive ordering
    pub fn get_pointer(
        &self,
        object_length: u32,
        allocator: &Function,
    ) -> Result<i32, MemoryAllocationError> {
        if self.length.load(Ordering::SeqCst) >= object_length {
            return Ok(self.pointer.load(Ordering::SeqCst));
        }
        let pointer = allocator
            .call(&[Value::I32(object_length as i32)])
            .map_err(MemoryAllocationError::CantAllocate)?;
        let pointer = pointer[0]
            .i32()
            .ok_or(MemoryAllocationError::InvalidReturnType)?;
        self.length.store(object_length, Ordering::SeqCst);
        self.pointer.store(pointer, Ordering::SeqCst);
        Ok(pointer)
    }

    // This function writes an object to WASM memory returning a pointer and a
    // length. Will realloc the buffer is not wide enough
    pub fn write_data<T: Serialize>(
        &self,
        memory: &Memory,
        allocator: &Function,
        object: &T,
    ) -> Result<(i32, u32), PluginModuleError> {
        self.write_bytes(
            memory,
            allocator,
            &bincode::serialize(object).map_err(PluginModuleError::Encoding)?,
        )
    }

    // This function writes an raw bytes to WASM memory returning a pointer and a
    // length. Will realloc the buffer is not wide enough
    pub fn write_bytes(
        &self,
        memory: &Memory,
        allocator: &Function,
        array: &[u8],
    ) -> Result<(i32, u32), PluginModuleError> {
        let len = array.len();
        let mem_position = self
            .get_pointer(len as u32, allocator)
            .map_err(PluginModuleError::MemoryAllocation)? as usize;
        memory.view()[mem_position..mem_position + len]
            .iter()
            .zip(array.iter())
            .for_each(|(cell, byte)| cell.set(*byte));
        Ok((mem_position as i32, len as u32))
    }
}

// This function read data from memory at a position with the array length and
// converts it to an object using bincode
pub fn read_data<T: DeserializeOwned>(
    memory: &Memory,
    position: i32,
    length: u32,
) -> Result<T, bincode::Error> {
    bincode::deserialize(&read_bytes(memory, position, length))
}

// This function read raw bytes from memory at a position with the array length
pub fn read_bytes(memory: &Memory, position: i32, length: u32) -> Vec<u8> {
    memory.view()[(position as usize)..(position as usize) + length as usize]
        .iter()
        .map(|x| x.get())
        .collect::<Vec<_>>()
}
