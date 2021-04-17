mod editable;

pub use editable::EditableSetting;

use authc::Uuid;
use hashbrown::{HashMap, HashSet};
use portpicker::pick_unused_port;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::Duration,
};
use tracing::{error, warn};
use world::sim::FileOpts;

const DEFAULT_WORLD_SEED: u32 = 25269;
const CONFIG_DIR: &str = "server_config";
const SETTINGS_FILENAME: &str = "settings.ron";
const WHITELIST_FILENAME: &str = "whitelist.ron";
const BANLIST_FILENAME: &str = "banlist.ron";
const SERVER_DESCRIPTION_FILENAME: &str = "description.ron";
const ADMINS_FILENAME: &str = "admins.ron";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub gameserver_address: SocketAddr,
    pub metrics_address: SocketAddr,
    pub auth_server_address: Option<String>,
    pub max_players: usize,
    pub world_seed: u32,
    //pub pvp_enabled: bool,
    pub server_name: String,
    pub start_time: f64,
    /// When set to None, loads the default map file (if available); otherwise,
    /// uses the value of the file options to decide how to proceed.
    pub map_file: Option<FileOpts>,
    pub max_view_distance: Option<u32>,
    pub banned_words_files: Vec<PathBuf>,
    pub max_player_group_size: u32,
    pub client_timeout: Duration,
    pub spawn_town: Option<String>,
    pub safe_spawn: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gameserver_address: SocketAddr::from(([0; 4], 14004)),
            metrics_address: SocketAddr::from(([0; 4], 14005)),
            auth_server_address: Some("https://auth.veloren.net".into()),
            world_seed: DEFAULT_WORLD_SEED,
            server_name: "Veloren Alpha".into(),
            max_players: 100,
            start_time: 9.0 * 3600.0,
            map_file: None,
            max_view_distance: Some(65),
            banned_words_files: Vec::new(),
            max_player_group_size: 6,
            client_timeout: Duration::from_secs(40),
            spawn_town: None,
            safe_spawn: true,
        }
    }
}

impl Settings {
    /// path: Directory that contains the server config directory
    pub fn load(path: &Path) -> Self {
        let path = Self::get_settings_path(path);

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader(file) {
                Ok(x) => x,
                Err(e) => {
                    let default_settings = Self::default();
                    let template_path = path.with_extension("template.ron");
                    warn!(
                        ?e,
                        "Failed to parse setting file! Falling back to default settings and \
                         creating a template file for you to migrate your current settings file: \
                         {}",
                        template_path.display()
                    );
                    if let Err(e) = default_settings.save_to_file(&template_path) {
                        error!(?e, "Failed to create template settings file")
                    }
                    default_settings
                },
            }
        } else {
            let default_settings = Self::default();

            if let Err(e) = default_settings.save_to_file(&path) {
                error!(?e, "Failed to create default settings file!");
            }
            default_settings
        }
    }

    fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        // Create dir if it doesn't exist
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let ron = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("Failed serialize settings.");

        fs::write(path, ron.as_bytes())?;

        Ok(())
    }

    /// path: Directory that contains the server config directory
    pub fn singleplayer(path: &Path) -> Self {
        let load = Self::load(&path);
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
            max_players: 100,
            start_time: 9.0 * 3600.0,
            max_view_distance: None,
            client_timeout: Duration::from_secs(180),
            ..load // Fill in remaining fields from server_settings.ron.
        }
    }

    fn get_settings_path(path: &Path) -> PathBuf {
        let mut path = with_config_dir(path);
        path.push(SETTINGS_FILENAME);
        path
    }
}

fn with_config_dir(path: &Path) -> PathBuf {
    let mut path = PathBuf::from(path);
    path.push(CONFIG_DIR);
    path
}

#[derive(Deserialize, Serialize)]
pub struct BanRecord {
    pub username_when_banned: String,
    pub reason: String,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(transparent)]
pub struct Whitelist(HashSet<Uuid>);

#[derive(Deserialize, Serialize, Default)]
#[serde(transparent)]
pub struct Banlist(HashMap<Uuid, BanRecord>);

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct ServerDescription(String);
impl Default for ServerDescription {
    fn default() -> Self { Self("This is the best Veloren server".into()) }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(transparent)]
pub struct Admins(HashSet<Uuid>);

/// Combines all the editable settings into one struct that is stored in the ecs
pub struct EditableSettings {
    pub whitelist: Whitelist,
    pub banlist: Banlist,
    pub server_description: ServerDescription,
    pub admins: Admins,
}

impl EditableSettings {
    pub fn load(data_dir: &Path) -> Self {
        Self {
            whitelist: Whitelist::load(data_dir),
            banlist: Banlist::load(data_dir),
            server_description: ServerDescription::load(data_dir),
            admins: Admins::load(data_dir),
        }
    }

    pub fn singleplayer(data_dir: &Path) -> Self {
        let load = Self::load(data_dir);
        Self {
            server_description: ServerDescription("Who needs friends anyway?".into()),
            // TODO: Let the player choose if they want to use admin commands or not
            admins: Admins(
                std::iter::once(crate::login_provider::derive_singleplayer_uuid()).collect(),
            ),
            ..load
        }
    }
}

impl EditableSetting for Whitelist {
    const FILENAME: &'static str = WHITELIST_FILENAME;
}

impl EditableSetting for Banlist {
    const FILENAME: &'static str = BANLIST_FILENAME;
}

impl EditableSetting for ServerDescription {
    const FILENAME: &'static str = SERVER_DESCRIPTION_FILENAME;
}

impl EditableSetting for Admins {
    const FILENAME: &'static str = ADMINS_FILENAME;
}

impl Deref for Whitelist {
    type Target = HashSet<Uuid>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Whitelist {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Deref for Banlist {
    type Target = HashMap<Uuid, BanRecord>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Banlist {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Deref for ServerDescription {
    type Target = String;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for ServerDescription {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Deref for Admins {
    type Target = HashSet<Uuid>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Admins {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
