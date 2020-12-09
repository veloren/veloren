use super::*;

// API
pub extern "C" fn on_tick() { PLUGIN.on_tick.lock().iter().for_each(|f| f()); }
