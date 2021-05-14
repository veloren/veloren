use directories_next::UserDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
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
    // The path on which the logs will be stored
    pub logs_path: PathBuf,
}

impl Default for Log {
    fn default() -> Self {
        // Chooses a path to store the logs by the following order:
        //  - The VOXYGEN_LOGS environment variable
        //  - The ProjectsDirs data local directory
        // This function is only called if there isn't already an entry in the settings
        // file. However, the VOXYGEN_LOGS environment variable always overrides
        // the log file path if set.
        let logs_path = std::env::var_os("VOXYGEN_LOGS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut path = voxygen_data_dir();
                path.push("logs");
                path
            });

        Self {
            log_to_file: true,
            logs_path,
        }
    }
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
    pub fn load() -> Self {
        let path = Self::get_settings_path();

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader::<_, Self>(file) {
                Ok(mut s) => {
                    // Override the logs path if it is explicitly set using the VOXYGEN_LOGS
                    // environment variable. This is needed to support package managers that enforce
                    // strict application confinement (e.g. snap). In fact, the veloren snap package
                    // relies on this environment variable to be respected in
                    // order to communicate a path where the snap package is
                    // allowed to write to.
                    if let Some(logs_path_override) =
                        std::env::var_os("VOXYGEN_LOGS").map(PathBuf::from)
                    {
                        s.log.logs_path = logs_path_override;
                    }
                    return s;
                },
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
        default_settings.save_to_file_warn();
        default_settings
    }

    pub fn save_to_file_warn(&self) {
        if let Err(e) = self.save_to_file() {
            warn!(?e, "Failed to save settings");
        }
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = Self::get_settings_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let ron = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        fs::write(path, ron.as_bytes())
    }

    pub fn get_settings_path() -> PathBuf {
        if let Some(path) = std::env::var_os("VOXYGEN_CONFIG") {
            let settings = PathBuf::from(&path).join("settings.ron");
            if settings.exists() || settings.parent().map(|x| x.exists()).unwrap_or(false) {
                return settings;
            }
            warn!(?path, "VOXYGEN_CONFIG points to invalid path.");
        }

        let mut path = voxygen_data_dir();
        path.push("settings.ron");
        path
    }
}

pub fn voxygen_data_dir() -> PathBuf {
    // Note: since voxygen is technically a lib we made need to lift this up to
    // run.rs
    let mut path = common_base::userdata_dir_workspace!();
    path.push("voxygen");
    path
}
