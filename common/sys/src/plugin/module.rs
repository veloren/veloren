use std::{
    cell::Cell,
    collections::HashSet,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use error::RuntimeError;
use wasmer_runtime::*;

use super::errors::{MemoryAllocationError, PluginError, PluginModuleError};
use plugin_api::{Action, Event};

// This represent a WASM function interface
pub type Function<'a> = Func<'a, (i32, u32), i32>;

#[derive(Clone)]
// This structure represent the WASM State of the plugin.
pub struct PluginModule {
    wasm_instance: Arc<Mutex<Instance>>,
    events: HashSet<String>,
}

impl PluginModule {
    // This function takes bytes from a WASM File and compile them
    pub fn new(wasm_data: &[u8]) -> Result<Self, PluginModuleError> {
        let module = compile(&wasm_data).map_err(PluginModuleError::Compile)?;
        let instance = module
            .instantiate(&imports! {"env" => {
                "raw_emit_actions" => func!(read_action),
            }})
            .map_err(PluginModuleError::Instantiate)?;

        Ok(Self {
            events: instance.exports.into_iter().map(|(name, _)| name).collect(),
            wasm_instance: Arc::new(Mutex::new(instance)),
        })
    }

    // This function tries to execute an event for the current module. Will return
    // None if the event doesn't exists
    pub fn try_execute<T>(
        &self,
        event_name: &str,
        request: &PreparedEventQuery<T>,
    ) -> Option<Result<T::Response, PluginModuleError>>
    where
        T: Event,
    {
        if !self.events.contains(event_name) {
            return None;
        }
        let bytes = {
            let instance = self.wasm_instance.lock().unwrap();
            let func = match instance
                .exports
                .get(event_name)
                .map_err(PluginModuleError::FunctionGet)
            {
                Ok(e) => e,
                Err(e) => return Some(Err(e)),
            };
            match execute_raw(&instance, &func, &request.bytes).map_err(PluginModuleError::RunFunction) {
                Ok(e) => e,
                Err(e) => return Some(Err(e)),
            }
        };
        Some(bincode::deserialize(&bytes).map_err(PluginModuleError::Encoding))
    }
}

// This structure represent a Pre-encoded event object (Useful to avoid
// reencoding for each module in every plugin)
pub struct PreparedEventQuery<T> {
    bytes: Vec<u8>,
    _phantom: PhantomData<T>,
}

impl<T: Event> PreparedEventQuery<T> {
    // Create a prepared query from an event reference (Encode to bytes the struct)
    // This Prepared Query is used by the `try_execute` method in `PluginModule`
    pub fn new(event: &T) -> Result<Self, PluginError>
    where
        T: Event,
    {
        Ok(Self {
            bytes: bincode::serialize(&event)
                .map_err(|e| PluginError::PluginModuleError(PluginModuleError::Encoding(e)))?,
            _phantom: PhantomData::default(),
        })
    }
}

// This function is not public because this function should not be used without
// an interface to limit unsafe behaviours
#[allow(clippy::needless_range_loop)]
fn execute_raw(
    instance: &Instance,
    function: &Function,
    bytes: &[u8],
) -> Result<Vec<u8>, RuntimeError> {
    // This reserves space for the buffer
    let len = bytes.len();
    let start = {
        let memory_pos = reserve_wasm_memory_buffer(len,instance).expect("Fatal error while allocating memory for a plugin! Closing server...") as usize;
        let memory = instance.context().memory(0);
        let view = memory.view::<u8>();
        for (cell, byte) in view[memory_pos..memory_pos+len].iter().zip(bytes.iter()) {
            cell.set(*byte)
        }
        function.call(memory_pos as i32, len as u32)? as usize
    };
    
    let memory = instance.context().memory(0);
    let view = memory.view::<u8>();
    let mut new_len_bytes = [0u8; 4];
    // TODO: It is probably better to dirrectly make the new_len_bytes
    for i in 0..4 {
        new_len_bytes[i] = view.get(i + 1).map(Cell::get).unwrap_or(0);
    }
    let new_len = u32::from_ne_bytes(new_len_bytes) as usize;
    Ok(view[start..start + new_len]
        .iter()
        .map(|c| c.get())
        .collect())
}

pub fn read_action(ctx: &mut Ctx, ptr: u32, len: u32) {
    let memory = ctx.memory(0);

    let memory = memory.view::<u8>();

    let str_slice = &memory[ptr as usize..(ptr + len) as usize];

    let bytes: Vec<u8> = str_slice.iter().map(|x| x.get()).collect();

    let e: Vec<Action> = match bincode::deserialize(&bytes) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(?e, "Can't decode action");
            return;
        },
    };

    for action in e {
        match action {
            Action::ServerClose => {
                tracing::info!("Server closed by plugin");
                std::process::exit(-1);
            },
            Action::Print(e) => {
                tracing::info!("{}", e);
            },
            Action::PlayerSendMessage(a, b) => {
                tracing::info!("SendMessage {} -> {}", a, b);
            },
            Action::KillEntity(e) => {
                tracing::info!("Kill Entity {}", e);
            },
        }
    }
}

fn reserve_wasm_memory_buffer<'a>(
    value: usize,
    instance: &'a Instance,
) -> Result<i32, MemoryAllocationError> {
    instance
        .exports
        .get::<Func<'a, i32, i32>>("wasm_prepare_buffer")
        .map_err(|e| MemoryAllocationError::AllocatorNotFound(e))?
        .call(value as i32)
        .map_err(|e| MemoryAllocationError::CantAllocate(e))
}