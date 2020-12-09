use plugin_api::{PLUGIN, api};

pub extern fn main() {
    PLUGIN.on_start(|| {
        api::print("Hello from my plugin!");
    });
}
