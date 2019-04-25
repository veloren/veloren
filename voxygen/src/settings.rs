use config::{Config, ConfigError};
use directories::ProjectDirs;
use glutin::VirtualKeyCode;
use serde_derive::{Deserialize, Serialize};
use std::{fs::File, io::prelude::*, path::PathBuf};
use toml;

/// Settings contains everything that can be configured in the Settings.toml file
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub controls: ControlSettings,
    pub networking: NetworkingSettings,
    pub log: Log,
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
    pub toggle_interface: VirtualKeyCode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkingSettings {
    pub username: String,
    pub servers: Vec<String>,
    pub default_server: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Log {
    pub file: PathBuf,
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
                toggle_interface: VirtualKeyCode::F2,
            },
            networking: NetworkingSettings {
                username: "Username".to_string(),
                servers: vec!["server.veloren.net".to_string()],
                default_server: 0,
            },
            log: Log {
                file: "voxygen.log".into(),
            },
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Config::new();

        config.merge(
            Config::try_from(&Settings::default())
                .expect("Default settings struct could not be converted to Config"),
        );

        let path = Settings::get_settings_path();

        config.merge::<config::File<config::FileSourceFile>>(path.into())?;

        config.try_into()
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = Settings::get_settings_path();

        let mut config_file = File::create(path)?;
        let s: &str = &toml::to_string_pretty(self).unwrap();
        config_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }

    fn get_settings_path() -> PathBuf {
        let proj_dirs =
            ProjectDirs::from("net", "veloren", "voxygen").expect("No home directory defined.");
        let path = proj_dirs.config_dir();
        path.join("settings");
        let path = path.with_extension("toml");
        path
    }
}
