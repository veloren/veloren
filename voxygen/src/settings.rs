use config::{
    Config,
    ConfigError,
};
use serde_derive::{Serialize, Deserialize};

use glutin::VirtualKeyCode;

/// Settings contains everything that can be configured in the Settings.toml file
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub controls: ControlSettings,
}

/// ControlSettings contains keybindings
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
    pub fn default() -> Self {
        Settings {
            controls: ControlSettings {
                toggle_cursor: VirtualKeyCode::Tab,
                escape: VirtualKeyCode::Escape,
                enter: VirtualKeyCode::Return,
                move_forward: VirtualKeyCode::W,
                move_left: VirtualKeyCode::A,
                move_back: VirtualKeyCode::S,
                move_right: VirtualKeyCode::D,
                map: VirtualKeyCode::M,
                bag: VirtualKeyCode::B,
                quest_log: VirtualKeyCode::L,
                character_window: VirtualKeyCode::C,
                social: VirtualKeyCode::O,
                spellbook: VirtualKeyCode::P,
                settings: VirtualKeyCode::N,
                help: VirtualKeyCode::F2,
                interface: VirtualKeyCode::F2,
            },
        }
    }
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Config::new();
        config.merge(config::File::with_name("Settings"))?;
        config.try_into()
    }
}
