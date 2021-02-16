use std::{collections::HashSet, convert::TryInto, marker::PhantomData, sync::{Arc, Mutex, atomic::AtomicI32}};

use specs::World;
use wasmer::{
    imports, Cranelift, Function, Instance, Memory, Module,
    Store, Value, JIT,
};

use super::{errors::{PluginError, PluginModuleError}, memory_manager::{self, MemoryManager}, wasm_env::HostFunctionEnvironement};

use plugin_api::{Action, Event};

#[derive(Clone)]
// This structure represent the WASM State of the plugin.
pub struct PluginModule {
    ecs: Arc<AtomicI32>,
    wasm_state: Arc<Mutex<Instance>>,
    memory_manager: Arc<MemoryManager>,
    events: HashSet<String>,
    allocator: Function,
    memory: Memory,
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
        fn raw_emit_actions(env: &HostFunctionEnvironement, ptr: u32, len: u32) {
            handle_actions(match env.read_data(ptr as i32, len) {
                Ok(e) => e,
                Err(e) => {
                    tracing::error!(?e, "Can't decode action");
                    return;
                },
            });
        }

        let ecs = Arc::new(AtomicI32::new(i32::MAX));
        let memory_manager = Arc::new(MemoryManager::new());

        // Create an import object.
        let import_object = imports! {
            "env" => {
                "raw_emit_actions" => Function::new_native_with_env(&store, HostFunctionEnvironement::new(name.clone(), ecs.clone(),memory_manager.clone()), raw_emit_actions),
            }
        };

        // Create an instance (Code execution environement)
        let instance = Instance::new(&module, &import_object)
            .map_err(PluginModuleError::InstantiationError)?;
        Ok(Self {
            memory_manager,
            ecs,
            memory: instance.exports.get_memory("memory").map_err(PluginModuleError::MemoryUninit)?.clone(),
            allocator: instance.exports.get_function("wasm_prepare_buffer").map_err(PluginModuleError::MemoryUninit)?.clone(),
            events: instance
                .exports
                .iter()
                .map(|(name, _)| name.to_string())
                .collect(),
            wasm_state: Arc::new(Mutex::new(instance)),
            name,
        })
    }

    // This function tries to execute an event for the current module. Will return
    // None if the event doesn't exists
    pub fn try_execute<T>(
        &self,
        ecs: &World,
        event_name: &str,
        request: &PreparedEventQuery<T>,
    ) -> Option<Result<T::Response, PluginModuleError>>
    where
        T: Event,
    {
        if !self.events.contains(event_name) {
            return None;
        }
        // Store the ECS Pointer for later use in `retreives`
        self.ecs.store((&ecs) as *const _ as i32, std::sync::atomic::Ordering::SeqCst);
        let bytes = {
            let mut state = self.wasm_state.lock().unwrap();
            match execute_raw(self,&mut state,event_name,&request.bytes) {
                Ok(e) => e,
                Err(e) => return Some(Err(e)),
            }
        };
        // Remove the ECS Pointer to avoid UB
        self.ecs.store(i32::MAX, std::sync::atomic::Ordering::SeqCst);
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
            bytes: bincode::serialize(&event).map_err(PluginError::Encoding)?,
            _phantom: PhantomData::default(),
        })
    }
}

fn from_i64(i: i64) -> (i32,i32) {
    let i = i.to_le_bytes();
    (i32::from_le_bytes(i[0..4].try_into().unwrap()),i32::from_le_bytes(i[4..8].try_into().unwrap()))
}

// This function is not public because this function should not be used without
// an interface to limit unsafe behaviours
#[allow(clippy::needless_range_loop)]
fn execute_raw(
    module: &PluginModule,
    instance: &mut Instance,
    event_name: &str,
    bytes: &[u8],
) -> Result<Vec<u8>, PluginModuleError> {

    // This write into memory `bytes` using allocation if necessary returning a pointer and a length

    let (mem_position,len) = module.memory_manager.write_bytes(&module.memory, &module.allocator, bytes)?;

    // This gets the event function from module exports

    let func = instance
        .exports
        .get_function(event_name)
        .map_err(PluginModuleError::MemoryUninit)?;

    // We call the function with the pointer and the length

    let function_result = func
        .call(&[Value::I32(mem_position as i32), Value::I32(len as i32)])
        .map_err(PluginModuleError::RunFunction)?;
    
    // Waiting for `multi-value` to be added to LLVM. So we encode the two i32 as an i64

    let (pointer,length) = from_i64(function_result[0]
        .i64()
        .ok_or_else(PluginModuleError::InvalidArgumentType)?);

    // We read the return object and deserialize it

    Ok(memory_manager::read_bytes(&module.memory, pointer, length as u32))
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
