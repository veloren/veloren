use config::{Config, ConfigError};
use directories::ProjectDirs;
use glutin::VirtualKeyCode;
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, path::PathBuf};
use toml;

/// `Settings` contains everything that can be configured in the Settings.toml file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub controls: ControlSettings,
    pub networking: NetworkingSettings,
    pub log: Log,
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
}

/// `ControlSettings` contains keybindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlSettings {
    pub toggle_cursor: VirtualKeyCode,
    pub escape: VirtualKeyCode,
    pub enter: VirtualKeyCode,
    pub move_forward: VirtualKeyCode,
    pub move_left: VirtualKeyCode,
    pub move_back: VirtualKeyCode,
    pub move_right: VirtualKeyCode,
    pub jump: VirtualKeyCode,
    pub glide: VirtualKeyCode,
    pub map: VirtualKeyCode,
    pub bag: VirtualKeyCode,
    pub quest_log: VirtualKeyCode,
    pub character_window: VirtualKeyCode,
    pub social: VirtualKeyCode,
    pub spellbook: VirtualKeyCode,
    pub settings: VirtualKeyCode,
    pub help: VirtualKeyCode,
    pub toggle_interface: VirtualKeyCode,
    pub toggle_debug: VirtualKeyCode,
    pub fullscreen: VirtualKeyCode,
    pub screenshot: VirtualKeyCode,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub view_distance: u32,
}

/// AudioSettings controls the volume of different audio subsystems and which
/// device is used.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioSettings {
    pub music_volume: f32,
    pub sfx_volume: f32,

    /// Audio Device that Voxygen will use to play audio.
    pub audio_device: Option<String>,
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
                jump: VirtualKeyCode::Space,
                glide: VirtualKeyCode::LShift,
                map: VirtualKeyCode::M,
                bag: VirtualKeyCode::B,
                quest_log: VirtualKeyCode::L,
                character_window: VirtualKeyCode::C,
                social: VirtualKeyCode::O,
                spellbook: VirtualKeyCode::P,
                settings: VirtualKeyCode::N,
                help: VirtualKeyCode::F1,
                toggle_interface: VirtualKeyCode::F2,
                toggle_debug: VirtualKeyCode::F3,
                fullscreen: VirtualKeyCode::F11,
                screenshot: VirtualKeyCode::F4,
            },
            networking: NetworkingSettings {
                username: "Username".to_string(),
                servers: vec!["server.veloren.net".to_string()],
                default_server: 0,
            },
            log: Log {
                file: "voxygen.log".into(),
            },
            graphics: GraphicsSettings { view_distance: 5 },
            audio: AudioSettings {
                music_volume: 0.5,
                sfx_volume: 0.5,
                audio_device: None,
            },
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let default_settings = Settings::default();

        let path = Settings::get_settings_path();

        let mut config = Config::new();

        config
            .merge(
                Config::try_from(&default_settings)
                    .expect("Default settings struct could not be converted to Config!"),
            )
            .unwrap();

        // TODO: Log errors here.
        // If merge or try_into fail, use the default settings.
        match config.merge::<config::File<_>>(path.into()) {
            Ok(_) => match config.try_into() {
                Ok(settings) => settings,
                Err(_) => default_settings,
            },
            Err(_) => {
                // Maybe the file didn't exist.
                // TODO: Handle this result.
                default_settings.save_to_file();
                default_settings
            }
        }
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = Settings::get_settings_path();

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let mut config_file = fs::File::create(path)?;
        let s: &str = &toml::to_string_pretty(self).unwrap();
        config_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }

    fn get_settings_path() -> PathBuf {
        let proj_dirs =
            ProjectDirs::from("net", "veloren", "voxygen").expect("No home directory defined!");
        let path = proj_dirs.config_dir();
        path.join("settings");
        let path = path.with_extension("toml");
        path
    }
}
