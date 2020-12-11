use std::{marker::PhantomData, sync::Arc};

use error::RuntimeError;
use parking_lot::Mutex;
use wasmer_runtime::*;

use super::errors::{PluginError, PluginModuleError};
use common_api::{Action, Event};

// This represent a WASM function interface
pub type Function<'a> = Func<'a, (i32, u32), i32>;

#[derive(Clone)]
// This tructure represent the WASM State of the plugin.
pub struct PluginModule {
    wasm_instance: Arc<Mutex<Instance>>,
    events: Vec<String>,
}

impl PluginModule {

    // This function take bytes from a WASM File and compile them
    pub fn new(wasm_data: &Vec<u8>) -> Result<Self,PluginModuleError> {
        let module = compile(&wasm_data).map_err(|e| PluginModuleError::Compile(e))?;
        let instance = module
            .instantiate(&imports! {"env" => {
                "send_action" => func!(read_action),
            }}).map_err(|e| PluginModuleError::Instantiate(e))?;
        
        Ok(Self {
            events: instance.exports.into_iter().map(|(name, _)| name).collect(),
            wasm_instance: Arc::new(Mutex::new(instance)),
        })
    }

    // This function try to execute an event for the current module will return None if the event doesn't exists
    pub fn try_execute<T>(
        &self,
        event_name: &str,
        request: &PreparedEventQuery<T>,
    ) -> Option<Result<T::Response,PluginModuleError>>
    where
        T: Event,
    {
        if !self.events.iter().any(|x| x == event_name) {
            return None;
        }
        let bytes = {
            let instance = self.wasm_instance.lock();
            let func = match instance.exports.get(event_name).map_err(|e| PluginModuleError::FunctionGet(e)) {
                Ok(e) => e,
                Err(e) => return Some(Err(e))
            };
            let mem = instance.context().memory(0);
            match execute_raw(&mem, &func, &request.bytes).map_err(|e| PluginModuleError::RunFunction(e)) {
                Ok(e) => e,
                Err(e) => return Some(Err(e))
            }
        };
        Some(bincode::deserialize(&bytes).map_err(|e| PluginModuleError::Encoding(e)))
    }
}

// This structure represent a Pre-encoded event object (Usefull to avoid reencoding for each module in every plugin)
pub struct PreparedEventQuery<T> {
    bytes: Vec<u8>,
    _phantom: PhantomData<T>,
}

impl<T: Event> PreparedEventQuery<T> {
    // Create a prepared query from a event reference (Encode to bytes the struct)
    // This Prepared Query is used by the `try_execute` method in `PluginModule`
    pub fn new(event: &T) -> Result<Self, PluginError>
    where
        T: Event,
    {
        Ok(Self {
            bytes: bincode::serialize(&event).map_err(|e| PluginError::PluginModuleError(PluginModuleError::Encoding(e)))?,
            _phantom: PhantomData::default(),
        })
    }
}

const MEMORY_POS: usize = 100000;

// This function is not public because this function should not be used without an interface to limit unsafe behaviours
#[allow(unsafe_code)]
fn execute_raw(
    memory: &Memory,
    function: &Function,
    bytes: &[u8],
) -> Result<Vec<u8>, RuntimeError> {
    let view = memory.view::<u8>();
    let len = bytes.len();
    for (cell, byte) in view[MEMORY_POS..len + MEMORY_POS].iter().zip(bytes.iter()) {
        cell.set(*byte)
    }
    let start = function
        .call(MEMORY_POS as i32, len as u32)? as usize;
    let view = memory.view::<u8>();
    let mut new_len_bytes = [0u8; 4];
    // TODO: It is probably better to dirrectly make the new_len_bytes
    for i in 0..4 {
        // Since memory view is a more than 11500 elements array we can get the [1;4] without any bound checks
        unsafe {
            new_len_bytes[i] = view.get_unchecked(i + 1).get();
        }
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
        }
    };

    for action in e {
        match action {
            Action::ServerClose => {
                tracing::info!("Server closed by plugin");
                std::process::exit(-1);
            }
            Action::Print(e) => {
                tracing::info!("{}",e);
            }
            Action::PlayerSendMessage(a, b) => {
                tracing::info!("SendMessage {} -> {}",a,b);
            }
            Action::KillEntity(e) => {
                tracing::info!("Kill Entity {}",e);
            }
        }
    }
}