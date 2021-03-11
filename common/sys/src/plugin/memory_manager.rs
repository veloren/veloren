use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering};

use serde::{de::DeserializeOwned, Serialize};
use specs::{
    storage::GenericReadStorage, Component, Entities, Entity, Read, ReadStorage, WriteStorage,
};
use wasmer::{Function, Memory, Value};

use common::{
    comp::{Health, Player},
    uid::{Uid, UidAllocator},
};

use super::errors::{MemoryAllocationError, PluginModuleError};

pub struct EcsWorld<'a, 'b> {
    pub entities: &'b Entities<'a>,
    pub health: EcsComponentAccess<'a, 'b, Health>,
    pub uid: EcsComponentAccess<'a, 'b, Uid>,
    pub player: EcsComponentAccess<'a, 'b, Player>,
    pub uid_allocator: &'b Read<'a, UidAllocator>,
}

pub enum EcsComponentAccess<'a, 'b, T: Component> {
    Read(&'b ReadStorage<'a, T>),
    ReadOwned(ReadStorage<'a, T>),
    Write(&'b WriteStorage<'a, T>),
    WriteOwned(WriteStorage<'a, T>),
}

impl<'a, 'b, T: Component> EcsComponentAccess<'a, 'b, T> {
    pub fn get(&self, entity: Entity) -> Option<&T> {
        match self {
            EcsComponentAccess::Read(e) => e.get(entity),
            EcsComponentAccess::Write(e) => e.get(entity),
            EcsComponentAccess::ReadOwned(e) => e.get(entity),
            EcsComponentAccess::WriteOwned(e) => e.get(entity),
        }
    }
}

impl<'a, 'b, T: Component> From<&'b ReadStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: &'b ReadStorage<'a, T>) -> Self { Self::Read(a) }
}

impl<'a, 'b, T: Component> From<ReadStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: ReadStorage<'a, T>) -> Self { Self::ReadOwned(a) }
}

impl<'a, 'b, T: Component> From<&'b WriteStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: &'b WriteStorage<'a, T>) -> Self { Self::Write(a) }
}

impl<'a, 'b, T: Component> From<WriteStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: WriteStorage<'a, T>) -> Self { Self::WriteOwned(a) }
}

// pub enum EcsResourceAccess<'a, T> {
//     Read(Read<'a, T>),
// }

/// This structure wraps the ECS pointer to ensure safety
pub struct EcsAccessManager {
    ecs_pointer: AtomicPtr<EcsWorld<'static, 'static>>,
}

impl Default for EcsAccessManager {
    fn default() -> Self {
        Self {
            ecs_pointer: AtomicPtr::new(std::ptr::null_mut()),
        }
    }
}

impl EcsAccessManager {
    // This function take a World reference and a function to execute ensuring the
    // pointer will never be corrupted during the execution of the function!
    pub fn execute_with<T>(&self, world: &EcsWorld, func: impl FnOnce() -> T) -> T {
        let _guard = scopeguard::guard((), |_| {
            // ensure the pointer is cleared in any case
            self.ecs_pointer
                .store(std::ptr::null_mut(), Ordering::Relaxed);
        });
        self.ecs_pointer
            .store(world as *const _ as *mut _, Ordering::Relaxed);
        func()
    }

    /// This unsafe function returns a reference to the Ecs World
    ///
    /// # Safety
    /// This function is safe to use if it matches the following requirements
    ///  - The reference and subreferences like Entities, Components ... aren't
    ///    leaked out the thread
    ///  - The reference and subreferences lifetime doesn't exceed the source
    ///    function lifetime
    ///  - Always safe when called from `retrieve_action` if you don't pass a
    ///    reference somewhere else
    ///  - All that ensure that the reference doesn't exceed the execute_with
    ///    function scope
    pub unsafe fn get(&self) -> Option<&EcsWorld> {
        // ptr::as_ref will automatically check for null
        self.ecs_pointer.load(Ordering::Relaxed).as_ref()
    }
}

pub struct MemoryManager {
    pub pointer: AtomicU64,
    pub length: AtomicU32,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            pointer: AtomicU64::new(0),
            length: AtomicU32::new(0),
        }
    }
}

impl MemoryManager {
    /// This function check if the buffer is wide enough if not it realloc the
    /// buffer calling the `wasm_prepare_buffer` function Note: There is
    /// probably optimizations that can be done using less restrictive
    /// ordering
    pub fn get_pointer(
        &self,
        object_length: u32,
        allocator: &Function,
    ) -> Result<u64, MemoryAllocationError> {
        if self.length.load(Ordering::SeqCst) >= object_length {
            return Ok(self.pointer.load(Ordering::SeqCst));
        }
        let pointer = allocator
            .call(&[Value::I32(object_length as i32)])
            .map_err(MemoryAllocationError::CantAllocate)?;
        let pointer = super::module::from_i64(
            pointer[0]
                .i64()
                .ok_or(MemoryAllocationError::InvalidReturnType)?,
        );
        self.length.store(object_length, Ordering::SeqCst);
        self.pointer.store(pointer, Ordering::SeqCst);
        Ok(pointer)
    }

    /// This function writes an object to WASM memory returning a pointer and a
    /// length. Will realloc the buffer is not wide enough
    pub fn write_data<T: Serialize>(
        &self,
        memory: &Memory,
        allocator: &Function,
        object: &T,
    ) -> Result<(u64, u64), PluginModuleError> {
        self.write_bytes(
            memory,
            allocator,
            &bincode::serialize(object).map_err(PluginModuleError::Encoding)?,
        )
    }

    /// This function writes an object to the wasm memory using the allocator if
    /// necessary using length padding.
    ///
    /// With length padding the first 8 bytes written are the length of the the
    /// following slice (The object serialized).
    pub fn write_data_as_pointer<T: Serialize>(
        &self,
        memory: &Memory,
        allocator: &Function,
        object: &T,
    ) -> Result<u64, PluginModuleError> {
        self.write_bytes_as_pointer(
            memory,
            allocator,
            &bincode::serialize(object).map_err(PluginModuleError::Encoding)?,
        )
    }

    /// This function writes an raw bytes to WASM memory returning a pointer and
    /// a length. Will realloc the buffer is not wide enough
    pub fn write_bytes(
        &self,
        memory: &Memory,
        allocator: &Function,
        bytes: &[u8],
    ) -> Result<(u64, u64), PluginModuleError> {
        let len = bytes.len();
        let mem_position = self
            .get_pointer(len as u32, allocator)
            .map_err(PluginModuleError::MemoryAllocation)? as usize;
        memory.view()[mem_position..mem_position + len]
            .iter()
            .zip(bytes.iter())
            .for_each(|(cell, byte)| cell.set(*byte));
        Ok((mem_position as u64, len as u64))
    }

    /// This function writes bytes to the wasm memory using the allocator if
    /// necessary using length padding.
    ///
    /// With length padding the first 8 bytes written are the length of the the
    /// following slice.
    pub fn write_bytes_as_pointer(
        &self,
        memory: &Memory,
        allocator: &Function,
        bytes: &[u8],
    ) -> Result<u64, PluginModuleError> {
        let len = bytes.len();
        let mem_position = self
            .get_pointer(len as u32 + 8, allocator)
            .map_err(PluginModuleError::MemoryAllocation)? as usize;
        // Here we write the length as le bytes followed by the slice data itself in
        // WASM memory
        memory.view()[mem_position..mem_position + len + 8]
            .iter()
            .zip((len as u64).to_le_bytes().iter().chain(bytes.iter()))
            .for_each(|(cell, byte)| cell.set(*byte));
        Ok(mem_position as u64)
    }
}

/// This function read data from memory at a position with the array length and
/// converts it to an object using bincode
pub fn read_data<T: DeserializeOwned>(
    memory: &Memory,
    position: u64,
    length: u64,
) -> Result<T, bincode::Error> {
    bincode::deserialize(&read_bytes(memory, position, length))
}

/// This function read raw bytes from memory at a position with the array length
pub fn read_bytes(memory: &Memory, position: u64, length: u64) -> Vec<u8> {
    memory.view()[(position as usize)..(position as usize) + length as usize]
        .iter()
        .map(|x| x.get())
        .collect()
}
