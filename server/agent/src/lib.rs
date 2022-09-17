#[cfg(all(feature = "be-dyn-lib", feature = "use-dyn-lib"))]
compile_error!("Can't use both \"be-dyn-lib\" and \"use-dyn-lib\" features at once");

#[cfg(all(target_os = "windows", feature = "be-dyn-lib"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod action_nodes;
pub mod attack;
pub mod consts;
pub mod data;
pub mod util;

#[cfg(feature = "use-dyn-lib")]
use {lazy_static::lazy_static, server_dynlib::LoadedLib, std::sync::Arc, std::sync::Mutex};

#[cfg(feature = "use-dyn-lib")]
lazy_static! {
    pub static ref LIB: Arc<Mutex<Option<LoadedLib>>> =
        server_dynlib::init("veloren-server-agent", "veloren-server-agent-dyn", "agent");
}

#[cfg(feature = "use-dyn-lib")]
pub fn init() { lazy_static::initialize(&LIB); }
