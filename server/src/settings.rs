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
    pub admins: Vec<String>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            address: SocketAddr::from(([0; 4], 14004)),
            world_seed: 1337,
            server_name: "Veloren Alpha".to_owned(),
            server_description: "This is the best Veloren server.".to_owned(),
            max_players: 100,
            start_time: 9.0 * 3600.0,
            admins: vec!["Pfau".to_owned()],
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

    pub fn singleplayer() -> Self {
        Self {
            address: SocketAddr::from(([0; 4], 14004)),
            world_seed: 1337,
            server_name: "Singleplayer".to_owned(),
            server_description: "Who needs friends anyway?".to_owned(),
            max_players: 100,
            start_time: 9.0 * 3600.0,
            admins: vec!["singleplayer".to_string()], // TODO: Let the player choose if they want to use admin commands or not
        }
    }

    fn get_settings_path() -> PathBuf {
        PathBuf::from(r"settings.ron")
    }
}
