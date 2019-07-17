use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub address: SocketAddr,
    pub max_players: usize,
    pub world_seed: u32,
    //pub pvp_enabled: bool,
    pub server_name: String,
    pub server_description: String,
    //pub login_server: whatever
    pub start_time: f64,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            address: SocketAddr::from(([0; 4], 59003)),
            world_seed: 1337,
            server_name: "Server name".to_owned(),
            server_description: "This is the best Veloren server.".to_owned(),
            max_players: 16,
            start_time: 0.0,
        }
    }
}

impl ServerSettings {
    pub fn load() -> Self {
        let path = ServerSettings::get_settings_path();

        if let Ok(file) = fs::File::open(path) {
            match ron::de::from_reader(file) {
                Ok(x) => x,
                Err(e) => {
                    log::warn!("Failed to parse setting file! Fallback to default. {}", e);
                    Self::default()
                }
            }
        } else {
            let default_settings = Self::default();

            match default_settings.save_to_file() {
                Err(e) => log::error!("Failed to create default setting file! {}", e),
                _ => {}
            }
            default_settings
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
