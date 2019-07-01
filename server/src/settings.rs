use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, net::SocketAddr, path::PathBuf};

/// `ControlSettings` contains keybindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub address: SocketAddr,
    //pub max_players: u64,
    pub world_seed: u32,
    //pub pvp_enabled: bool,
    pub server_name: String,
    pub server_description: String,
    //pub login_server: whatever
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            address: SocketAddr::from(([0; 4], 59003)),
            world_seed: 1337,
            server_name: "Server name".to_owned(),
            server_description: "This is the best Veloren server.".to_owned(),
        }
    }
}

impl ServerSettings {
    pub fn load() -> Self {
        let path = ServerSettings::get_settings_path();

        // If file doesn't exist, use the default settings.
        if let Ok(file) = fs::File::open(path) {
            // TODO: Replace expect with returning default?
            ron::de::from_reader(file).expect("Error parsing settings")
        } else {
            // TODO: temporary
            Self::default().save_to_file().unwrap();
            Self::default()
        }
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = ServerSettings::get_settings_path();
        let mut config_file = fs::File::create(path)?;

        let s: &str = &ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        config_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }

    fn get_settings_path() -> PathBuf {
        PathBuf::from(r"settings.ron")
    }
}
