use std::{
    cell::Cell,
    collections::HashSet,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use wasmer::{
    imports, Cranelift, Function, HostEnvInitError, Instance, LazyInit, Memory, MemoryView, Module,
    Store, Value, WasmerEnv, JIT,
};

use super::errors::{MemoryAllocationError, PluginError, PluginModuleError};

use plugin_api::{Action, Event};

#[derive(Clone)]
// This structure represent the WASM State of the plugin.
pub struct PluginModule {
    wasm_state: Arc<Mutex<WasmState>>,
    events: HashSet<String>,
    name: String,
}

impl PluginModule {
    // This function takes bytes from a WASM File and compile them
    pub fn new(name: String, wasm_data: &[u8]) -> Result<Self, PluginModuleError> {
        // This is creating the engine is this case a JIT based on Cranelift
        let engine = JIT::new(Cranelift::default()).engine();
        // We are creating an enironnement
        let store = Store::new(&engine);
        // We are compiling the WASM file in the previously generated environement
        let module = Module::new(&store, &wasm_data).expect("Can't compile");

        // This is the function imported into the wasm environement
        fn raw_emit_actions(env: &EmitActionEnv, ptr: u32, len: u32) {
            let memory: &Memory = if let Some(e) = env.memory.get_ref() {
                e
            } else {
                // This should not be possible but I prefer be safer!
                tracing::error!("Can't get memory from: `{}` plugin", env.name);
                return;
            };
            let memory: MemoryView<u8> = memory.view();

            let str_slice = &memory[ptr as usize..(ptr + len) as usize];

            let bytes: Vec<u8> = str_slice.iter().map(|x| x.get()).collect();

            handle_actions(match bincode::deserialize(&bytes) {
                Ok(e) => e,
                Err(e) => {
                    tracing::error!(?e, "Can't decode action");
                    return;
                },
            });
        }

        // Create an import object.
        let import_object = imports! {
            "env" => {
                "raw_emit_actions" => Function::new_native_with_env(&store, EmitActionEnv::new(name.clone()), raw_emit_actions),
            }
        };

        // Create an instance (Code execution environement)
        let instance = Instance::new(&module, &import_object)
            .map_err(PluginModuleError::InstantiationError)?;
        Ok(Self {
            events: instance
                .exports
                .iter()
                .map(|(name, _)| name.to_string())
                .collect(),
            wasm_state: Arc::new(Mutex::new(WasmState::new(instance))),
            name,
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
            let mut state = self.wasm_state.lock().unwrap();
            match execute_raw(&mut state, event_name, &request.bytes) {
                Ok(e) => e,
                Err(e) => return Some(Err(e)),
            }
        };
        Some(bincode::deserialize(&bytes).map_err(PluginModuleError::Encoding))
    }
}

/// This is an internal struct used to represent the WASM state when the
/// emit_action function is called
#[derive(Clone)]
struct EmitActionEnv {
    memory: LazyInit<Memory>,
    name: String,
}

impl EmitActionEnv {
    fn new(name: String) -> Self {
        Self {
            memory: LazyInit::new(),
            name,
        }
    }
}

impl WasmerEnv for EmitActionEnv {
    fn init_with_instance(&mut self, instance: &Instance) -> Result<(), HostEnvInitError> {
        let memory = instance.exports.get_memory("memory").unwrap();
        self.memory.initialize(memory.clone());
        Ok(())
    }
}

pub struct WasmMemoryContext {
    memory_buffer_size: usize,
    memory_pointer: i32,
}

pub struct WasmState {
    instance: Instance,
    memory: WasmMemoryContext,
}

impl WasmState {
    fn new(instance: Instance) -> Self {
        Self {
            instance,
            memory: WasmMemoryContext {
                memory_buffer_size: 0,
                memory_pointer: 0,
            },
        }
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
            bytes: bincode::serialize(&event).map_err(PluginError::Encoding)?,
            _phantom: PhantomData::default(),
        })
    }
}

// This function is not public because this function should not be used without
// an interface to limit unsafe behaviours
#[allow(clippy::needless_range_loop)]
fn execute_raw(
    instance: &mut WasmState,
    event_name: &str,
    bytes: &[u8],
) -> Result<Vec<u8>, PluginModuleError> {
    let len = bytes.len();

    let mem_position = reserve_wasm_memory_buffer(len, &instance.instance, &mut instance.memory)
        .map_err(PluginModuleError::MemoryAllocation)? as usize;

    let memory = instance
        .instance
        .exports
        .get_memory("memory")
        .map_err(PluginModuleError::MemoryUninit)?;

    memory.view()[mem_position..mem_position + len]
        .iter()
        .zip(bytes.iter())
        .for_each(|(cell, byte)| cell.set(*byte));

    let func = instance
        .instance
        .exports
        .get_function(event_name)
        .map_err(PluginModuleError::MemoryUninit)?;

    let mem_position = func
        .call(&[Value::I32(mem_position as i32), Value::I32(len as i32)])
        .map_err(PluginModuleError::RunFunction)?[0]
        .i32()
        .ok_or_else(PluginModuleError::InvalidArgumentType)? as usize;

    let view: MemoryView<u8> = memory.view();

    let mut new_len_bytes = [0u8; 4];
    // TODO: It is probably better to dirrectly make the new_len_bytes
    for i in 0..4 {
        new_len_bytes[i] = view.get(i + 1).map(Cell::get).unwrap_or(0);
    }

    let len = u32::from_ne_bytes(new_len_bytes) as usize;

    Ok(view[mem_position..mem_position + len]
        .iter()
        .map(|x| x.get())
        .collect())
}

fn reserve_wasm_memory_buffer(
    size: usize,
    instance: &Instance,
    context: &mut WasmMemoryContext,
) -> Result<i32, MemoryAllocationError> {
    if context.memory_buffer_size >= size {
        return Ok(context.memory_pointer);
    }
    let pointer = instance
        .exports
        .get_function("wasm_prepare_buffer")
        .map_err(MemoryAllocationError::AllocatorNotFound)?
        .call(&[Value::I32(size as i32)])
        .map_err(MemoryAllocationError::CantAllocate)?;
    context.memory_buffer_size = size;
    context.memory_pointer = pointer[0].i32().unwrap();
    Ok(context.memory_pointer)
}

fn handle_actions(actions: Vec<Action>) {
    for action in actions {
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
