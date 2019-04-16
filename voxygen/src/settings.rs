use config::{
    Config,
    ConfigError,
};
use serde_derive::{Serialize, Deserialize};
use glutin::VirtualKeyCode;
use toml;
use std::fs::File;
use std::io::prelude::*;
use std::default::Default;

/// Settings contains everything that can be configured in the Settings.toml file
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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

impl Default for Settings {
    fn default() -> Self {
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
                help: VirtualKeyCode::F1,
                interface: VirtualKeyCode::F2,
            },
        }
    }

}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Config::new();
        config.merge(Config::try_from(&Settings::default())?);
        config.merge(config::File::with_name("settings"))?;
        config.try_into()
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let mut config_file = File::create("settings.toml")?;
        let s: &str = &toml::to_string_pretty(self).unwrap();
        config_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }
}
