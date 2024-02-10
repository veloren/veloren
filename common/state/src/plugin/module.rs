use std::sync::Arc;

use super::{
    errors::{EcsAccessError, PluginModuleError},
    memory_manager::{EcsAccessManager, EcsWorld},
    CommandResults,
};
use hashbrown::HashSet;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::WasiView;

wasmtime::component::bindgen!({
    path: "../../plugin/wit/veloren.wit",
    async: true,
    with: {
        "veloren:plugin/information@0.0.1/entity": Entity,
    },
});

pub struct Entity {
    uid: common::uid::Uid,
}

use veloren::plugin::{actions, information, types};

/// This structure represent the WASM State of the plugin.
pub struct PluginModule {
    ecs: Arc<EcsAccessManager>,
    plugin: Plugin,
    store: wasmtime::Store<WasiHostCtx>,
    #[allow(dead_code)]
    name: String,
}

struct WasiHostCtx {
    preview2_ctx: wasmtime_wasi::preview2::WasiCtx,
    preview2_table: wasmtime::component::ResourceTable,
    ecs: Arc<EcsAccessManager>,
    registered_commands: HashSet<String>,
}

impl wasmtime_wasi::preview2::WasiView for WasiHostCtx {
    fn table(&self) -> &wasmtime::component::ResourceTable { &self.preview2_table }

    fn ctx(&self) -> &wasmtime_wasi::preview2::WasiCtx { &self.preview2_ctx }

    fn table_mut(&mut self) -> &mut wasmtime::component::ResourceTable { &mut self.preview2_table }

    fn ctx_mut(&mut self) -> &mut wasmtime_wasi::preview2::WasiCtx { &mut self.preview2_ctx }
}

impl information::Host for WasiHostCtx {}

impl types::Host for WasiHostCtx {}

#[wasmtime::component::__internal::async_trait]
impl actions::Host for WasiHostCtx {
    async fn register_command(&mut self, name: String) -> wasmtime::Result<()> {
        tracing::info!("Plugin registers /{name}");
        self.registered_commands.insert(name);
        Ok(())
    }

    async fn player_send_message(
        &mut self,
        uid: actions::Uid,
        text: String,
    ) -> wasmtime::Result<()> {
        tracing::info!("Plugin sends message {text} to player {uid:?}");
        Ok(())
    }
}

#[wasmtime::component::__internal::async_trait]
impl information::HostEntity for WasiHostCtx {
    async fn find_entity(
        &mut self,
        uid: actions::Uid,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<information::Entity>, ()>> {
        let entry = self.table_mut().push(Entity {
            uid: common::uid::Uid(uid),
        })?;
        Ok(Ok(entry))
    }

    async fn health(
        &mut self,
        self_: wasmtime::component::Resource<information::Entity>,
    ) -> wasmtime::Result<information::Health> {
        let uid = self.table().get(&self_)?.uid;
        // Safety: No reference is leaked out the function so it is safe.
        let world = unsafe {
            self.ecs
                .get()
                .ok_or(EcsAccessError::EcsPointerNotAvailable)?
        };
        let player = world
            .id_maps
            .uid_entity(uid)
            .ok_or(EcsAccessError::EcsEntityNotFound(uid))?;
        world
            .health
            .get(player)
            .map(|health| information::Health {
                current: health.current(),
                base_max: health.base_max(),
                maximum: health.maximum(),
            })
            .ok_or_else(|| EcsAccessError::EcsComponentNotFound(uid, "Health".to_owned()).into())
    }

    async fn name(
        &mut self,
        self_: wasmtime::component::Resource<information::Entity>,
    ) -> wasmtime::Result<String> {
        let uid = self.table().get(&self_)?.uid;
        // Safety: No reference is leaked out the function so it is safe.
        let world = unsafe {
            self.ecs
                .get()
                .ok_or(EcsAccessError::EcsPointerNotAvailable)?
        };
        let player = world
            .id_maps
            .uid_entity(uid)
            .ok_or(EcsAccessError::EcsEntityNotFound(uid))?;
        Ok(world
            .player
            .get(player)
            .ok_or_else(|| EcsAccessError::EcsComponentNotFound(uid, "Player".to_owned()))?
            .alias
            .to_owned())
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<information::Entity>,
    ) -> wasmtime::Result<()> {
        Ok(self.table_mut().delete(rep).map(|_entity| ())?)
    }
}

struct InfoStream(String);

impl wasmtime_wasi::preview2::HostOutputStream for InfoStream {
    fn write(&mut self, bytes: bytes::Bytes) -> wasmtime_wasi::preview2::StreamResult<()> {
        tracing::info!("{}: {}", self.0, String::from_utf8_lossy(bytes.as_ref()));
        Ok(())
    }

    fn flush(&mut self) -> wasmtime_wasi::preview2::StreamResult<()> { Ok(()) }

    fn check_write(&mut self) -> wasmtime_wasi::preview2::StreamResult<usize> { Ok(1024) }
}

#[async_trait::async_trait]
impl wasmtime_wasi::preview2::Subscribe for InfoStream {
    async fn ready(&mut self) {}
}

struct ErrorStream(String);

impl wasmtime_wasi::preview2::HostOutputStream for ErrorStream {
    fn write(&mut self, bytes: bytes::Bytes) -> wasmtime_wasi::preview2::StreamResult<()> {
        tracing::error!("{}: {}", self.0, String::from_utf8_lossy(bytes.as_ref()));
        Ok(())
    }

    fn flush(&mut self) -> wasmtime_wasi::preview2::StreamResult<()> { Ok(()) }

    fn check_write(&mut self) -> wasmtime_wasi::preview2::StreamResult<usize> { Ok(1024) }
}

#[async_trait::async_trait]
impl wasmtime_wasi::preview2::Subscribe for ErrorStream {
    async fn ready(&mut self) {}
}

struct LogStream(String, tracing::Level);

impl wasmtime_wasi::preview2::StdoutStream for LogStream {
    fn stream(&self) -> Box<dyn wasmtime_wasi::preview2::HostOutputStream> {
        if self.1 == tracing::Level::INFO {
            Box::new(InfoStream(self.0.clone()))
        } else {
            Box::new(ErrorStream(self.0.clone()))
        }
    }

    fn isatty(&self) -> bool { true }
}

impl PluginModule {
    /// This function takes bytes from a WASM File and compile them
    pub fn new(name: String, wasm_data: &[u8]) -> Result<Self, PluginModuleError> {
        let ecs = Arc::new(EcsAccessManager::default());

        // configure the wasm runtime
        let mut config = Config::new();
        config.async_support(true).wasm_component_model(true);

        let engine = Engine::new(&config).map_err(PluginModuleError::Wasmtime)?;
        // create a WASI environment (std implementing system calls)
        let wasi = wasmtime_wasi::preview2::WasiCtxBuilder::new()
            .stdout(LogStream(name.clone(), tracing::Level::INFO))
            .stderr(LogStream(name.clone(), tracing::Level::ERROR))
            .build();
        let host_ctx = WasiHostCtx {
            preview2_ctx: wasi,
            preview2_table: wasmtime_wasi::preview2::ResourceTable::new(),
            ecs: Arc::clone(&ecs),
            registered_commands: HashSet::new(),
        };
        // the store contains all data of a wasm instance
        let mut store = Store::new(&engine, host_ctx);

        // load wasm from binary
        let module =
            Component::from_binary(&engine, wasm_data).map_err(PluginModuleError::Wasmtime)?;

        // register WASI and Veloren methods with the runtime
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::preview2::command::add_to_linker(&mut linker)
            .map_err(PluginModuleError::Wasmtime)?;
        Plugin::add_to_linker(&mut linker, |x| x).map_err(PluginModuleError::Wasmtime)?;

        let instance_fut = Plugin::instantiate_async(&mut store, &module, &linker);
        let (plugin, _instance) =
            futures::executor::block_on(instance_fut).map_err(PluginModuleError::Wasmtime)?;

        Ok(Self {
            plugin,
            ecs,
            store,
            name,
        })
    }

    pub fn name(&self) -> &str { &self.name }

    // Implementation of the commands called from veloren and provided in plugins
    pub fn load_event(
        &mut self,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<(), PluginModuleError> {
        let mode = match mode {
            common::resources::GameMode::Server => types::GameMode::Server,
            common::resources::GameMode::Client => types::GameMode::Client,
            common::resources::GameMode::Singleplayer => types::GameMode::SinglePlayer,
        };
        self.ecs
            .execute_with(ecs, || {
                let future = self
                    .plugin
                    .veloren_plugin_events()
                    .call_load(&mut self.store, mode);
                futures::executor::block_on(future)
            })
            .map_err(PluginModuleError::Wasmtime)
    }

    pub fn command_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        args: &[String],
        player: common::uid::Uid,
    ) -> Result<Vec<String>, CommandResults> {
        if !self.store.data().registered_commands.contains(name) {
            return Err(CommandResults::UnknownCommand);
        }
        self.ecs.execute_with(ecs, || {
            let future = self.plugin.veloren_plugin_events().call_command(
                &mut self.store,
                name,
                args,
                player.0,
            );
            match futures::executor::block_on(future) {
                Err(err) => Err(CommandResults::HostError(err)),
                Ok(result) => result.map_err(CommandResults::PluginError),
            }
        })
    }

    pub fn player_join_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        uuid: common::uuid::Uuid,
    ) -> types::JoinResult {
        self.ecs.execute_with(ecs, || {
            let future = self.plugin.veloren_plugin_events().call_join(
                &mut self.store,
                name,
                uuid.as_u64_pair(),
            );
            match futures::executor::block_on(future) {
                Ok(value) => {
                    tracing::info!("JoinResult {value:?}");
                    value
                },
                Err(err) => {
                    tracing::error!("join_event: {err:?}");
                    types::JoinResult::None
                },
            }
        })
    }
}
