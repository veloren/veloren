#![feature(const_fn)]

pub mod api;
pub mod raw_api;
pub mod raw_hooks;

use spin::Mutex;

#[derive(Copy, Clone, Debug)]
pub enum Hook {
    OnStart,
    OnTick,
    OnStop,
}

pub struct Plugin {
    pub on_start: Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
    pub on_tick: Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
    pub on_stop: Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
}

impl Plugin {
    pub const fn new() -> Self {
        Self {
            on_start: Mutex::new(Vec::new()),
            on_tick: Mutex::new(Vec::new()),
            on_stop: Mutex::new(Vec::new()),
        }
    }

    pub fn on_start(&self, f: impl Fn() + Send + Sync + 'static) {
        self.on_start.lock().push(Box::new(f));
    }
}

pub static PLUGIN: Plugin = Plugin::new();
