use super::BotCreds;
use std::{fs, path::PathBuf};
use tracing::warn;

pub fn data_dir() -> PathBuf {
    let mut path = common_base::userdata_dir_workspace!();
    path.push("botclient");
    path
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server: String,
    pub bot_logins: Vec<BotCreds>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            server: "localhost".to_string(),
            bot_logins: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Self::get_settings_path();

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader(file) {
                Ok(s) => return s,
                Err(e) => {
                    warn!(?e, "Failed to parse setting file! Fallback to default.");
                    // Rename the corrupted settings file
                    let mut new_path = path.to_owned();
                    new_path.pop();
                    new_path.push("settings.invalid.ron");
                    if let Err(e) = fs::rename(&path, &new_path) {
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

    fn save_to_file(&self) -> std::io::Result<()> {
        let path = Self::get_settings_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let ron = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        fs::write(path, ron.as_bytes())
    }

    pub fn get_settings_path() -> PathBuf {
        let mut path = data_dir();
        path.push("settings.ron");
        path
    }
}
