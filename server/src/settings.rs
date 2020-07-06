use portpicker::pick_unused_port;
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, net::SocketAddr, path::PathBuf};
use tracing::{error, warn};
use world::sim::FileOpts;

const DEFAULT_WORLD_SEED: u32 = 59686;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub gameserver_address: SocketAddr,
    pub metrics_address: SocketAddr,
    pub auth_server_address: Option<String>,
    pub max_players: usize,
    pub world_seed: u32,
    //pub pvp_enabled: bool,
    pub server_name: String,
    pub server_description: String,
    pub start_time: f64,
    pub admins: Vec<String>,
    pub whitelist: Vec<String>,
    /// When set to None, loads the default map file (if available); otherwise,
    /// uses the value of the file options to decide how to proceed.
    pub map_file: Option<FileOpts>,
    pub persistence_db_dir: String,
    pub max_view_distance: Option<u32>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            gameserver_address: SocketAddr::from(([0; 4], 14004)),
            metrics_address: SocketAddr::from(([0; 4], 14005)),
            auth_server_address: Some("https://auth.veloren.net".into()),
            world_seed: DEFAULT_WORLD_SEED,
            server_name: "Veloren Alpha".to_owned(),
            server_description: "This is the best Veloren server.".to_owned(),
            max_players: 100,
            start_time: 9.0 * 3600.0,
            map_file: None,
            admins: [
                "Pfau",
                "zesterer",
                "xMAC94x",
                "Timo",
                "Songtronix",
                "slipped",
                "Sharp",
                "Acrimon",
                "imbris",
                "YuriMomo",
                "Vechro",
                "AngelOnFira",
                "Nancok",
                "Qutrin",
                "Mckol",
                "Treeco",
            ]
            .iter()
            .map(|n| n.to_string())
            .collect(),
            whitelist: Vec::new(),
            persistence_db_dir: "saves".to_owned(),
            max_view_distance: Some(30),
        }
    }
}

impl ServerSettings {
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    pub fn load() -> Self {
        let path = ServerSettings::get_settings_path();

        if let Ok(file) = fs::File::open(path) {
            match ron::de::from_reader(file) {
                Ok(x) => x,
                Err(e) => {
                    warn!(?e, "Failed to parse setting file! Fallback to default");
                    Self::default()
                },
            }
        } else {
            let default_settings = Self::default();

            match default_settings.save_to_file() {
                Err(e) => error!(?e, "Failed to create default setting file!"),
                _ => {},
            }
            default_settings
        }
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = ServerSettings::get_settings_path();
        let mut config_file = fs::File::create(path)?;

        let s: &str = &ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("Failed serialize settings.");
        config_file.write_all(s.as_bytes())?;
        Ok(())
    }

    pub fn singleplayer(persistence_db_dir: String) -> Self {
        let load = Self::load();
        Self {
            //BUG: theoretically another process can grab the port between here and server
            // creation, however the timewindow is quite small
            gameserver_address: SocketAddr::from((
                [127, 0, 0, 1],
                pick_unused_port().expect("Failed to find unused port!"),
            )),
            metrics_address: SocketAddr::from((
                [127, 0, 0, 1],
                pick_unused_port().expect("Failed to find unused port!"),
            )),
            auth_server_address: None,
            // If loading the default map file, make sure the seed is also default.
            world_seed: if load.map_file.is_some() {
                load.world_seed
            } else {
                DEFAULT_WORLD_SEED
            },
            server_name: "Singleplayer".to_owned(),
            server_description: "Who needs friends anyway?".to_owned(),
            max_players: 100,
            start_time: 9.0 * 3600.0,
            admins: vec!["singleplayer".to_string()], /* TODO: Let the player choose if they want
                                                       * to use admin commands or not */
            persistence_db_dir,
            max_view_distance: None,
            ..load // Fill in remaining fields from server_settings.ron.
        }
    }

    fn get_settings_path() -> PathBuf { PathBuf::from(r"server_settings.ron") }

    pub fn edit<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let r = f(self);
        self.save_to_file()
            .unwrap_or_else(|err| warn!("Failed to save settings: {:?}", err));
        r
    }
}
