use directories_next::UserDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::warn;

pub mod audio;
pub mod chat;
pub mod control;
pub mod gamepad;
pub mod gameplay;
pub mod graphics;
pub mod interface;
pub mod language;
pub mod networking;

pub use audio::{AudioOutput, AudioSettings};
pub use chat::ChatSettings;
pub use control::ControlSettings;
pub use gamepad::GamepadSettings;
pub use gameplay::GameplaySettings;
pub use graphics::{get_fps, Fps, GraphicsSettings};
pub use interface::InterfaceSettings;
pub use language::LanguageSettings;
pub use networking::NetworkingSettings;

/// `Log` stores whether we should create a log file
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Log {
    // Whether to create a log file or not.
    // Default is to create one.
    pub log_to_file: bool,
}

impl Default for Log {
    fn default() -> Self { Self { log_to_file: true } }
}

/// `Settings` contains everything that can be configured in the settings.ron
/// file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub chat: ChatSettings,
    pub controls: ControlSettings,
    pub interface: InterfaceSettings,
    pub gameplay: GameplaySettings,
    pub networking: NetworkingSettings,
    pub log: Log,
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
    pub show_disclaimer: bool,
    pub send_logon_commands: bool,
    // TODO: Remove at a later date, for dev testing
    pub logon_commands: Vec<String>,
    pub language: LanguageSettings,
    pub screenshots_path: PathBuf,
    pub controller: GamepadSettings,
}

impl Default for Settings {
    fn default() -> Self {
        let user_dirs = UserDirs::new().expect("System's $HOME directory path not found!");

        // Chooses a path to store the screenshots by the following order:
        //  - The VOXYGEN_SCREENSHOT environment variable
        //  - The user's picture directory
        //  - The executable's directory
        // This only selects if there isn't already an entry in the settings file
        let screenshots_path = std::env::var_os("VOXYGEN_SCREENSHOT")
            .map(PathBuf::from)
            .or_else(|| user_dirs.picture_dir().map(|dir| dir.join("veloren")))
            .or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|dir| dir.parent().map(PathBuf::from))
            })
            .expect("Couldn't choose a place to store the screenshots");

        Settings {
            chat: ChatSettings::default(),
            controls: ControlSettings::default(),
            interface: InterfaceSettings::default(),
            gameplay: GameplaySettings::default(),
            networking: NetworkingSettings::default(),
            log: Log::default(),
            graphics: GraphicsSettings::default(),
            audio: AudioSettings::default(),
            show_disclaimer: true,
            send_logon_commands: false,
            logon_commands: Vec::new(),
            language: LanguageSettings::default(),
            screenshots_path,
            controller: GamepadSettings::default(),
        }
    }
}

impl Settings {
    pub fn load(config_dir: &Path) -> Self {
        let path = Self::get_path(config_dir);

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader::<_, Self>(file) {
                Ok(s) => return s,
                Err(e) => {
                    warn!(?e, "Failed to parse setting file! Fallback to default.");
                    // Rename the corrupted settings file
                    let mut new_path = path.to_owned();
                    new_path.pop();
                    new_path.push("settings.invalid.ron");
                    if let Err(e) = std::fs::rename(&path, &new_path) {
                        warn!(?e, ?path, ?new_path, "Failed to rename settings file.");
                    }
                },
            }
        }
        // This is reached if either:
        // - The file can't be opened (presumably it doesn't exist)
        // - Or there was an error parsing the file
        let default_settings = Self::default();
        default_settings.save_to_file_warn(config_dir);
        default_settings
    }

    pub fn save_to_file_warn(&self, config_dir: &Path) {
        if let Err(e) = self.save_to_file(config_dir) {
            warn!(?e, "Failed to save settings");
        }
    }

    pub fn save_to_file(&self, config_dir: &Path) -> std::io::Result<()> {
        let path = Self::get_path(config_dir);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let ron = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        fs::write(path, ron.as_bytes())
    }

    fn get_path(config_dir: &Path) -> PathBuf { config_dir.join("settings.ron") }
}
