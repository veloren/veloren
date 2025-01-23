use std::sync::{Arc, Mutex};

use super::{
    CommandResults,
    errors::PluginModuleError,
    memory_manager::{EcsAccessManager, EcsWorld},
};
use hashbrown::{HashMap, HashSet};
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::WasiView;

pub(crate) mod types_mod {
    wasmtime::component::bindgen!({
        path: "../../plugin/wit/veloren.wit",
        world: "common-types",
    });
}

wasmtime::component::bindgen!({
    path: "../../plugin/wit/veloren.wit",
    world: "plugin",
    with: {
        "veloren:plugin/types@0.0.1": types_mod::veloren::plugin::types,
        "veloren:plugin/information@0.0.1/entity": Entity,
    },
});

mod animation_plugin {
    wasmtime::component::bindgen!({
        path: "../../plugin/wit/veloren.wit",
        world: "animation-plugin",
        with: {
            "veloren:plugin/types@0.0.1": super::types_mod::veloren::plugin::types,
        },
    });
}

mod server_plugin {
    wasmtime::component::bindgen!({
        path: "../../plugin/wit/veloren.wit",
        world: "server-plugin",
        with: {
            "veloren:plugin/types@0.0.1": super::types_mod::veloren::plugin::types,
            "veloren:plugin/information@0.0.1/entity": super::Entity,
        },
    });
}

pub struct Entity {
    uid: common::uid::Uid,
}

pub use animation::Body;
use exports::veloren::plugin::animation;
pub use types_mod::veloren::plugin::types::{
    self, CharacterState, Dependency, Skeleton, Transform,
};
use veloren::plugin::{actions, information};

type StoreType = wasmtime::Store<WasiHostCtx>;

/// This enum abstracts over the different types of plugins we defined
enum PluginWrapper {
    Full(Plugin),
    Animation(animation_plugin::AnimationPlugin),
    Server(server_plugin::ServerPlugin),
}

impl PluginWrapper {
    fn load_event<S: wasmtime::AsContextMut>(
        &self,
        store: S,
        mode: common::resources::GameMode,
    ) -> wasmtime::Result<()>
    where
        <S as wasmtime::AsContext>::Data: std::marker::Send,
    {
        let mode = match mode {
            common::resources::GameMode::Server => types::GameMode::Server,
            common::resources::GameMode::Client => types::GameMode::Client,
            common::resources::GameMode::Singleplayer => types::GameMode::SinglePlayer,
        };
        match self {
            PluginWrapper::Full(pl) => pl.veloren_plugin_events().call_load(store, mode),
            PluginWrapper::Animation(pl) => pl.veloren_plugin_events().call_load(store, mode),
            PluginWrapper::Server(pl) => pl.veloren_plugin_events().call_load(store, mode),
        }
    }

    fn command_event<S: wasmtime::AsContextMut>(
        &self,
        store: S,
        name: &str,
        args: &[String],
        player: types::Uid,
    ) -> wasmtime::Result<Result<Vec<String>, String>>
    where
        <S as wasmtime::AsContext>::Data: std::marker::Send,
    {
        match self {
            PluginWrapper::Full(pl) => pl
                .veloren_plugin_server_events()
                .call_command(store, name, args, player),
            PluginWrapper::Animation(_) => Ok(Err("not implemented".into())),
            PluginWrapper::Server(pl) => pl
                .veloren_plugin_server_events()
                .call_command(store, name, args, player),
        }
    }

    fn player_join_event(
        &self,
        store: &mut StoreType,
        name: &str,
        uuid: (types::Uid, types::Uid),
    ) -> wasmtime::Result<types::JoinResult> {
        match self {
            PluginWrapper::Full(pl) => pl
                .veloren_plugin_server_events()
                .call_join(store, name, uuid),
            PluginWrapper::Animation(_) => Ok(types::JoinResult::None),
            PluginWrapper::Server(pl) => pl
                .veloren_plugin_server_events()
                .call_join(store, name, uuid),
        }
    }

    fn create_body(&self, store: &mut StoreType, bodytype: i32) -> Option<animation::Body> {
        match self {
            PluginWrapper::Full(pl) => {
                let body_iface = pl.veloren_plugin_animation().body();
                body_iface.call_constructor(store, bodytype).ok()
            },
            PluginWrapper::Animation(pl) => {
                let body_iface = pl.veloren_plugin_animation().body();
                body_iface.call_constructor(store, bodytype).ok()
            },
            PluginWrapper::Server(_) => None,
        }
    }

    fn update_skeleton(
        &self,
        store: &mut StoreType,
        body: animation::Body,
        dep: types::Dependency,
        time: f32,
    ) -> Option<types::Skeleton> {
        match self {
            PluginWrapper::Full(pl) => {
                let body_iface = pl.veloren_plugin_animation().body();
                body_iface.call_update_skeleton(store, body, dep, time).ok()
            },
            PluginWrapper::Animation(pl) => {
                let body_iface = pl.veloren_plugin_animation().body();
                body_iface.call_update_skeleton(store, body, dep, time).ok()
            },
            PluginWrapper::Server(_) => None,
        }
    }
}

/// This structure represent the WASM State of the plugin.
pub struct PluginModule {
    ecs: Arc<EcsAccessManager>,
    plugin: PluginWrapper,
    store: Mutex<wasmtime::Store<WasiHostCtx>>,
    #[allow(dead_code)]
    name: String,
}

struct WasiHostCtx {
    preview2_ctx: wasmtime_wasi::WasiCtx,
    preview2_table: wasmtime::component::ResourceTable,
    ecs: Arc<EcsAccessManager>,
    registered_commands: HashSet<String>,
    registered_bodies: HashMap<String, types::BodyIndex>,
}

impl wasmtime_wasi::WasiView for WasiHostCtx {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable { &mut self.preview2_table }

    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx { &mut self.preview2_ctx }
}

impl information::Host for WasiHostCtx {}

impl types::Host for WasiHostCtx {}

impl actions::Host for WasiHostCtx {
    fn register_command(&mut self, name: String) {
        tracing::info!("Plugin registers /{name}");
        self.registered_commands.insert(name);
    }

    fn player_send_message(&mut self, uid: actions::Uid, text: String) {
        tracing::info!("Plugin sends message {text} to player {uid:?}");
    }

    fn register_animation(&mut self, name: String, id: types::BodyIndex) {
        let _ = self.registered_bodies.insert(name, id);
    }
}

impl information::HostEntity for WasiHostCtx {
    fn find_entity(
        &mut self,
        uid: actions::Uid,
    ) -> Result<wasmtime::component::Resource<information::Entity>, types::Error> {
        self.table()
            .push(Entity {
                uid: common::uid::Uid(uid),
            })
            .map_err(|_err| types::Error::RuntimeError)
    }

    fn health(
        &mut self,
        self_: wasmtime::component::Resource<information::Entity>,
    ) -> Result<information::Health, types::Error> {
        let uid = self
            .table()
            .get(&self_)
            .map_err(|_err| types::Error::RuntimeError)?
            .uid;
        // Safety: No reference is leaked out the function so it is safe.
        let world = unsafe { self.ecs.get().ok_or(types::Error::EcsPointerNotAvailable)? };
        let player = world
            .id_maps
            .uid_entity(uid)
            .ok_or(types::Error::EcsEntityNotFound)?;
        world
            .health
            .get(player)
            .map(|health| information::Health {
                current: health.current(),
                base_max: health.base_max(),
                maximum: health.maximum(),
            })
            .ok_or(types::Error::EcsComponentNotFound)
    }

    fn name(
        &mut self,
        self_: wasmtime::component::Resource<information::Entity>,
    ) -> Result<String, types::Error> {
        let uid = self
            .table()
            .get(&self_)
            .map_err(|_err| types::Error::RuntimeError)?
            .uid;
        // Safety: No reference is leaked out the function so it is safe.
        let world = unsafe { self.ecs.get().ok_or(types::Error::EcsPointerNotAvailable)? };
        let player = world
            .id_maps
            .uid_entity(uid)
            .ok_or(types::Error::EcsEntityNotFound)?;
        Ok(world
            .player
            .get(player)
            .ok_or(types::Error::EcsComponentNotFound)?
            .alias
            .to_owned())
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<information::Entity>,
    ) -> wasmtime::Result<()> {
        Ok(self.table().delete(rep).map(|_entity| ())?)
    }
}

struct InfoStream(String);

impl wasmtime_wasi::HostOutputStream for InfoStream {
    fn write(&mut self, bytes: bytes::Bytes) -> wasmtime_wasi::StreamResult<()> {
        tracing::info!("{}: {}", self.0, String::from_utf8_lossy(bytes.as_ref()));
        Ok(())
    }

    fn flush(&mut self) -> wasmtime_wasi::StreamResult<()> { Ok(()) }

    fn check_write(&mut self) -> wasmtime_wasi::StreamResult<usize> { Ok(1024) }
}

#[wasmtime_wasi::async_trait]
impl wasmtime_wasi::Subscribe for InfoStream {
    async fn ready(&mut self) {}
}

struct ErrorStream(String);

impl wasmtime_wasi::HostOutputStream for ErrorStream {
    fn write(&mut self, bytes: bytes::Bytes) -> wasmtime_wasi::StreamResult<()> {
        tracing::error!("{}: {}", self.0, String::from_utf8_lossy(bytes.as_ref()));
        Ok(())
    }

    fn flush(&mut self) -> wasmtime_wasi::StreamResult<()> { Ok(()) }

    fn check_write(&mut self) -> wasmtime_wasi::StreamResult<usize> { Ok(1024) }
}

#[wasmtime_wasi::async_trait]
impl wasmtime_wasi::Subscribe for ErrorStream {
    async fn ready(&mut self) {}
}

struct LogStream(String, tracing::Level);

impl wasmtime_wasi::StdoutStream for LogStream {
    fn stream(&self) -> Box<dyn wasmtime_wasi::HostOutputStream> {
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
        config.wasm_component_model(true);

        let engine = Engine::new(&config).map_err(PluginModuleError::Wasmtime)?;
        // create a WASI environment (std implementing system calls)
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .stdout(LogStream(name.clone(), tracing::Level::INFO))
            .stderr(LogStream(name.clone(), tracing::Level::ERROR))
            .build();
        let host_ctx = WasiHostCtx {
            preview2_ctx: wasi,
            preview2_table: wasmtime_wasi::ResourceTable::new(),
            ecs: Arc::clone(&ecs),
            registered_commands: HashSet::new(),
            registered_bodies: HashMap::new(),
        };
        // the store contains all data of a wasm instance
        let mut store = Store::new(&engine, host_ctx);

        // load wasm from binary
        let module =
            Component::from_binary(&engine, wasm_data).map_err(PluginModuleError::Wasmtime)?;

        // register WASI and Veloren methods with the runtime
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker).map_err(PluginModuleError::Wasmtime)?;
        Plugin::add_to_linker(&mut linker, |x| x).map_err(PluginModuleError::Wasmtime)?;

        let instance_fut = linker.instantiate(&mut store, &module);
        let instance = (instance_fut).map_err(PluginModuleError::Wasmtime)?;

        let plugin = match Plugin::new(&mut store, &instance) {
            Ok(pl) => Ok(PluginWrapper::Full(pl)),
            Err(_) => match animation_plugin::AnimationPlugin::new(&mut store, &instance) {
                Ok(pl) => Ok(PluginWrapper::Animation(pl)),
                Err(_) => server_plugin::ServerPlugin::new(&mut store, &instance)
                    .map(PluginWrapper::Server),
            },
        }
        .map_err(PluginModuleError::Wasmtime)?;

        Ok(Self {
            plugin,
            ecs,
            store: store.into(),
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
        self.ecs
            .execute_with(ecs, || {
                self.plugin.load_event(self.store.get_mut().unwrap(), mode)
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
        if !self
            .store
            .get_mut()
            .unwrap()
            .data()
            .registered_commands
            .contains(name)
        {
            return Err(CommandResults::UnknownCommand);
        }
        self.ecs.execute_with(ecs, || {
            match self
                .plugin
                .command_event(self.store.get_mut().unwrap(), name, args, player.0)
            {
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
            match self.plugin.player_join_event(
                self.store.get_mut().unwrap(),
                name,
                uuid.as_u64_pair(),
            ) {
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

    pub fn create_body(&mut self, bodytype: &str) -> Option<animation::Body> {
        let store = self.store.get_mut().unwrap();
        let bodytype = store.data().registered_bodies.get(bodytype).copied();
        bodytype.and_then(|bd| self.plugin.create_body(store, bd))
    }

    pub fn update_skeleton(
        &mut self,
        body: &animation::Body,
        dep: &types::Dependency,
        time: f32,
    ) -> Option<types::Skeleton> {
        self.plugin
            .update_skeleton(self.store.get_mut().unwrap(), *body, *dep, time)
    }
}
