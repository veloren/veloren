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
use {common_dynlib::LoadedLib, lazy_static::lazy_static, std::sync::Arc, std::sync::Mutex};

#[cfg(feature = "use-dyn-lib")]
lazy_static! {
    pub static ref LIB: Arc<Mutex<Option<LoadedLib>>> =
        common_dynlib::init("veloren-server-agent", "veloren-server-agent-dyn", "agent");
}

#[cfg(feature = "use-dyn-lib")]
pub fn init() { lazy_static::initialize(&LIB); }
