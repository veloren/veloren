use std::{
    collections::HashSet,
    convert::TryInto,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use common::{
    comp::{Health, Player},
    uid::UidAllocator,
};
use specs::{saveload::MarkerAllocator, World, WorldExt};
use wasmer::{imports, Cranelift, Function, Instance, Memory, Module, Store, Value, JIT};

use super::{
    errors::{PluginError, PluginModuleError},
    memory_manager::{self, EcsAccessManager, MemoryManager},
    wasm_env::HostFunctionEnvironement,
};

use plugin_api::{Action, EcsAccessError, Event, Retrieve, RetrieveError, RetrieveResult};

#[derive(Clone)]
// This structure represent the WASM State of the plugin.
pub struct PluginModule {
    ecs: Arc<EcsAccessManager>,
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

        fn raw_retrieve_action(env: &HostFunctionEnvironement, ptr: u32, len: u32) -> i64 {
            // TODO: Handle correctly the error
            let data: Retrieve = env.read_data(ptr as _, len).unwrap();

            let out = retrieve_action(&env.ecs, data);
            let (ptr, len) = env.write_data(&out).unwrap();
            to_i64(ptr, len as _)
        }

        fn dbg(a: i32) {
            println!("WASM DEBUG: {}", a);
        }

        let ecs = Arc::new(EcsAccessManager::default());
        let memory_manager = Arc::new(MemoryManager::default());

        // Create an import object.
        let import_object = imports! {
            "env" => {
                "raw_emit_actions" => Function::new_native_with_env(&store, HostFunctionEnvironement::new(name.clone(), ecs.clone(),memory_manager.clone()), raw_emit_actions),
                "raw_retrieve_action" => Function::new_native_with_env(&store, HostFunctionEnvironement::new(name.clone(), ecs.clone(),memory_manager.clone()), raw_retrieve_action),
                "dbg" => Function::new_native(&store, dbg),
            }
        };

        // Create an instance (Code execution environement)
        let instance = Instance::new(&module, &import_object)
            .map_err(PluginModuleError::InstantiationError)?;
        Ok(Self {
            memory_manager,
            ecs,
            memory: instance
                .exports
                .get_memory("memory")
                .map_err(PluginModuleError::MemoryUninit)?
                .clone(),
            allocator: instance
                .exports
                .get_function("wasm_prepare_buffer")
                .map_err(PluginModuleError::MemoryUninit)?
                .clone(),
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
        let bytes = match self.ecs.execute_with(ecs, || {
            let mut state = self.wasm_state.lock().unwrap();
            execute_raw(self, &mut state, event_name, &request.bytes)
        }) {
            Ok(e) => e,
            Err(e) => return Some(Err(e)),
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
            bytes: bincode::serialize(&event).map_err(PluginError::Encoding)?,
            _phantom: PhantomData::default(),
        })
    }
}

fn from_i64(i: i64) -> (i32, i32) {
    let i = i.to_le_bytes();
    (
        i32::from_le_bytes(i[0..4].try_into().unwrap()),
        i32::from_le_bytes(i[4..8].try_into().unwrap()),
    )
}

pub fn to_i64(a: i32, b: i32) -> i64 {
    let a = a.to_le_bytes();
    let b = b.to_le_bytes();
    i64::from_le_bytes([a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]])
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
    // This write into memory `bytes` using allocation if necessary returning a
    // pointer and a length

    let (mem_position, len) =
        module
            .memory_manager
            .write_bytes(&module.memory, &module.allocator, bytes)?;

    // This gets the event function from module exports

    let func = instance
        .exports
        .get_function(event_name)
        .map_err(PluginModuleError::MemoryUninit)?;

    // We call the function with the pointer and the length

    let function_result = func
        .call(&[Value::I32(mem_position as i32), Value::I32(len as i32)])
        .map_err(PluginModuleError::RunFunction)?;

    // Waiting for `multi-value` to be added to LLVM. So we encode the two i32 as an
    // i64

    let (pointer, length) = from_i64(
        function_result[0]
            .i64()
            .ok_or_else(PluginModuleError::InvalidArgumentType)?,
    );

    // We read the return object and deserialize it

    Ok(memory_manager::read_bytes(
        &module.memory,
        pointer,
        length as u32,
    ))
}

fn retrieve_action(
    ecs: &EcsAccessManager,
    action: Retrieve,
) -> Result<RetrieveResult, RetrieveError> {
    match action {
        Retrieve::GetPlayerName(e) => {
            let world = ecs.get().ok_or(RetrieveError::EcsAccessError(
                EcsAccessError::EcsPointerNotAvailable,
            ))?;
            let player = world
                .read_resource::<UidAllocator>()
                .retrieve_entity_internal(e.0)
                .ok_or(RetrieveError::EcsAccessError(
                    EcsAccessError::EcsEntityNotFound(e),
                ))?;
            Ok(RetrieveResult::GetPlayerName(
                world
                    .read_component::<Player>()
                    .get(player)
                    .ok_or_else(|| {
                        RetrieveError::EcsAccessError(EcsAccessError::EcsComponentNotFound(
                            e,
                            "Player".to_owned(),
                        ))
                    })?
                    .alias
                    .to_owned(),
            ))
        },
        Retrieve::GetEntityHealth(e) => {
            let world = ecs.get().ok_or(RetrieveError::EcsAccessError(
                EcsAccessError::EcsPointerNotAvailable,
            ))?;
            let player = world
                .read_resource::<UidAllocator>()
                .retrieve_entity_internal(e.0)
                .ok_or(RetrieveError::EcsAccessError(
                    EcsAccessError::EcsEntityNotFound(e),
                ))?;
            Ok(RetrieveResult::GetEntityHealth(
                *world
                    .read_component::<Health>()
                    .get(player)
                    .ok_or_else(|| {
                        RetrieveError::EcsAccessError(EcsAccessError::EcsComponentNotFound(
                            e,
                            "Health".to_owned(),
                        ))
                    })?,
            ))
        },
    }
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
