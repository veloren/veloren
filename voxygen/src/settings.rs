use config::{
    Config,
    ConfigError,
};
use serde_derive::{Serialize, Deserialize};

use glutin::VirtualKeyCode;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub controls: ControlSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlSettings {
    pub toggle_cursor: VirtualKeyCode,
    pub escape: VirtualKeyCode,
    pub enter: VirtualKeyCode,
    pub move_forward: VirtualKeyCode,
    pub move_left: VirtualKeyCode,
    pub move_back: VirtualKeyCode,
    pub move_right: VirtualKeyCode,
    pub map: VirtualKeyCode,
    pub bag: VirtualKeyCode,
    pub quest_log: VirtualKeyCode,
    pub character_window: VirtualKeyCode,
    pub social: VirtualKeyCode,
    pub spellbook: VirtualKeyCode,
    pub settings: VirtualKeyCode,
    pub help: VirtualKeyCode,
    pub interface: VirtualKeyCode,
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Config::new();
        config.merge(config::File::with_name("Settings"))?;
        config.try_into()
    }
}
