use super::BotCreds;
use std::{fs, path::PathBuf};
use tracing::warn;

pub fn data_dir() -> PathBuf {
    let mut path = common_base::userdata_dir_workspace!();
    path.push("botclient");
    path
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
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

        let settings = common::util::ron_from_path_recoverable::<Self>(&path);
        // Save settings to add new fields or create the file if it is not already there
        settings.save_to_file_warn();
        settings
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
