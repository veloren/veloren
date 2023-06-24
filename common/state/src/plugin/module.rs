use hashbrown::HashSet;
use std::{marker::PhantomData, sync::Arc};

use wasmer::{
    imports, AsStoreMut, AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Instance, Memory,
    Module, Store, TypedFunction, WasmPtr,
};

use super::{
    errors::{PluginError, PluginModuleError},
    exports,
    memory_manager::{self, EcsAccessManager, EcsWorld},
    wasm_env::HostFunctionEnvironment,
    MemoryModel,
};

use plugin_api::{Action, EcsAccessError, Event, Retrieve, RetrieveError, RetrieveResult};

// #[derive(Clone)]
/// This structure represent the WASM State of the plugin.
pub struct PluginModule {
    ecs: Arc<EcsAccessManager>,
    wasm_state: Arc<Instance>,
    events: HashSet<String>,
    allocator: TypedFunction<<MemoryModel as wasmer::MemorySize>::Offset, WasmPtr<u8, MemoryModel>>,
    memory: Memory,
    store: Store,
    #[allow(dead_code)]
    name: String,
    pub(crate) exit_code: Option<i32>,
}

impl PluginModule {
    /// This function takes bytes from a WASM File and compile them
    pub fn new(name: String, wasm_data: &[u8]) -> Result<Self, PluginModuleError> {
        // The store contains all data for a specific instance, including the linear
        // memory
        let mut store = Store::default();
        // We are compiling the WASM file in the previously generated environement
        let module = Module::from_binary(store.engine(), wasm_data)
            .map_err(PluginModuleError::CompileError)?;

        // This is the function imported into the wasm environement
        fn raw_emit_actions(
            env: FunctionEnvMut<HostFunctionEnvironment>,
            // store: &wasmer::StoreRef<'_>,
            ptr: WasmPtr<u8, MemoryModel>,
            len: <MemoryModel as wasmer::MemorySize>::Offset,
        ) {
            handle_actions(
                match env.data().read_serialized(&env.as_store_ref(), ptr, len) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!(?e, "Can't decode action");
                        return;
                    },
                },
            );
        }

        fn raw_retrieve_action(
            mut env: FunctionEnvMut<HostFunctionEnvironment>,
            // store: &wasmer::StoreRef<'_>,
            ptr: WasmPtr<u8, MemoryModel>,
            len: <MemoryModel as wasmer::MemorySize>::Offset,
        ) -> <MemoryModel as wasmer::MemorySize>::Offset {
            let out = match env.data().read_serialized(&env.as_store_ref(), ptr, len) {
                Ok(data) => retrieve_action(&env.data().ecs(), data),
                Err(e) => Err(RetrieveError::BincodeError(e.to_string())),
            };

            let data = env.data().clone();
            data.write_serialized_with_length(&mut env.as_store_mut(), &out)
                .unwrap_or_else(|_e|
                    // return a null pointer so the WASM side can tell an error occured
                    WasmPtr::null())
                .offset()
        }

        fn dbg(a: i32) {
            println!("WASM DEBUG: {}", a);
        }

        let ecs = Arc::new(EcsAccessManager::default());

        // Environment to pass ecs and memory_manager to callbacks
        let env = FunctionEnv::new(
            &mut store,
            HostFunctionEnvironment::new(name.clone(), Arc::clone(&ecs)),
        );
        // Create an import object.
        let import_object = imports! {
            "env" => {
                "raw_emit_actions" => Function::new_typed_with_env(&mut store, &env, raw_emit_actions),
                "raw_retrieve_action" => Function::new_typed_with_env(&mut store, &env, raw_retrieve_action),
                "dbg" => Function::new_typed(&mut store, dbg),
            },
            "wasi_snapshot_preview1" => {
                "fd_write" => Function::new_typed_with_env(&mut store, &env, exports::wasi_fd_write),
                "environ_get" => Function::new_typed_with_env(&mut store, &env, exports::wasi_env_get),
                "environ_sizes_get" => Function::new_typed_with_env(&mut store, &env, exports::wasi_env_sizes_get),
                "proc_exit" => Function::new_typed_with_env(&mut store, &env, exports::wasi_proc_exit),
            },
        };

        // Create an instance (Code execution environement)
        let instance = Instance::new(&mut store, &module, &import_object)
            .map_err(|err| PluginModuleError::InstantiationError(Box::new(err)))?;
        let init_args = HostFunctionEnvironment::args_from_instance(&store, &instance)
            .map_err(PluginModuleError::FindFunction)?;
        env.as_mut(&mut store).init_with_instance(init_args);
        Ok(Self {
            ecs,
            memory: instance
                .exports
                .get_memory("memory")
                .map_err(PluginModuleError::MemoryUninit)?
                .clone(),
            allocator: instance
                .exports
                .get_typed_function(&store, "wasm_prepare_buffer")
                .map_err(PluginModuleError::MemoryUninit)?,
            events: instance
                .exports
                .iter()
                .map(|(name, _)| name.to_string())
                .collect(),
            wasm_state: Arc::new(instance),
            store,
            name,
            exit_code: None,
        })
    }

    /// This function tries to execute an event for the current module. Will
    /// return None if the event doesn't exists
    pub fn try_execute<T>(
        &mut self,
        ecs: &EcsWorld,
        request: &PreparedEventQuery<T>,
    ) -> Option<Result<T::Response, PluginModuleError>>
    where
        T: Event,
    {
        if !self.events.contains(&request.function_name) {
            return None;
        }
        // Store the ECS Pointer for later use in `retreives`
        let s_ecs = self.ecs.clone();
        let bytes = match s_ecs.execute_with(ecs, || {
            execute_raw(self, &request.function_name, &request.bytes)
        }) {
            Ok(e) => e,
            Err(e) => return Some(Err(e)),
        };
        Some(bincode::deserialize(&bytes).map_err(PluginModuleError::Encoding))
    }

    pub fn name(&self) -> &str { &self.name }
}

/// This structure represent a Pre-encoded event object (Useful to avoid
/// reencoding for each module in every plugin)
pub struct PreparedEventQuery<T> {
    bytes: Vec<u8>,
    function_name: String,
    _phantom: PhantomData<T>,
}

impl<T: Event> PreparedEventQuery<T> {
    /// Create a prepared query from an event reference (Encode to bytes the
    /// struct) This Prepared Query is used by the `try_execute` method in
    /// `PluginModule`
    pub fn new(event: &T) -> Result<Self, PluginError>
    where
        T: Event,
    {
        Ok(Self {
            bytes: bincode::serialize(&event).map_err(PluginError::Encoding)?,
            function_name: event.get_event_name(),
            _phantom: PhantomData::default(),
        })
    }

    pub fn get_function_name(&self) -> &str { &self.function_name }
}

// This function is not public because this function should not be used without
// an interface to limit unsafe behaviours
fn execute_raw(
    module: &mut PluginModule,
    // instance: &mut Instance,
    event_name: &str,
    bytes: &[u8],
) -> Result<Vec<u8>, PluginModuleError> {
    // This write into memory `bytes` using allocation if necessary returning a
    // pointer and a length

    let (ptr, len) = memory_manager::write_bytes(
        &mut module.store.as_store_mut(),
        &module.memory,
        &module.allocator,
        (bytes, &[]),
    )?;

    // This gets the event function from module exports

    let func: TypedFunction<
        (
            WasmPtr<u8, MemoryModel>,
            <MemoryModel as wasmer::MemorySize>::Offset,
        ),
        WasmPtr<u8, MemoryModel>,
    > = module
        .wasm_state
        .exports
        .get_typed_function(&module.store.as_store_ref(), event_name)
        .map_err(PluginModuleError::MemoryUninit)?;

    // We call the function with the pointer and the length

    let result_ptr = func
        .call(&mut module.store.as_store_mut(), ptr, len)
        .map_err(PluginModuleError::RunFunction)?;

    // The first bytes correspond to the length of the result
    let result_len: [u8; std::mem::size_of::<<MemoryModel as wasmer::MemorySize>::Offset>()] =
        memory_manager::read_exact_bytes(&module.memory, &module.store.as_store_ref(), result_ptr)
            .map_err(|_| PluginModuleError::InvalidPointer)?;
    let result_len = <MemoryModel as wasmer::MemorySize>::Offset::from_le_bytes(result_len);

    // Read the result of the function with the pointer and the length
    let bytes = memory_manager::read_bytes(
        &module.memory,
        &module.store.as_store_ref(),
        WasmPtr::new(
            result_ptr.offset()
                + std::mem::size_of::<<MemoryModel as wasmer::MemorySize>::Offset>()
                    as <MemoryModel as wasmer::MemorySize>::Offset,
        ),
        result_len,
    )?;
    Ok(bytes)
}

fn retrieve_action(
    ecs: &EcsAccessManager,
    action: Retrieve,
) -> Result<RetrieveResult, RetrieveError> {
    match action {
        Retrieve::GetPlayerName(e) => {
            // Safety: No reference is leaked out the function so it is safe.
            let world = unsafe {
                ecs.get().ok_or(RetrieveError::EcsAccessError(
                    EcsAccessError::EcsPointerNotAvailable,
                ))?
            };
            let player = world
                .id_maps
                .uid_entity(e)
                .ok_or(RetrieveError::EcsAccessError(
                    EcsAccessError::EcsEntityNotFound(e),
                ))?;

            Ok(RetrieveResult::GetPlayerName(
                world
                    .player
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
            // Safety: No reference is leaked out the function so it is safe.
            let world = unsafe {
                ecs.get().ok_or(RetrieveError::EcsAccessError(
                    EcsAccessError::EcsPointerNotAvailable,
                ))?
            };
            let player = world
                .id_maps
                .uid_entity(e)
                .ok_or(RetrieveError::EcsAccessError(
                    EcsAccessError::EcsEntityNotFound(e),
                ))?;
            Ok(RetrieveResult::GetEntityHealth(
                world
                    .health
                    .get(player)
                    .ok_or_else(|| {
                        RetrieveError::EcsAccessError(EcsAccessError::EcsComponentNotFound(
                            e,
                            "Health".to_owned(),
                        ))
                    })?
                    .clone(),
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
