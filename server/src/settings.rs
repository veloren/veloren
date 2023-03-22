pub mod admin;
pub mod banlist;
mod editable;
pub mod server_description;
pub mod whitelist;

pub use editable::{EditableSetting, Error as SettingError};

pub use admin::{AdminRecord, Admins};
pub use banlist::{
    Ban, BanAction, BanEntry, BanError, BanErrorKind, BanInfo, BanKind, BanRecord, Banlist,
};
pub use server_description::ServerDescription;
pub use whitelist::{Whitelist, WhitelistInfo, WhitelistRecord};

use chrono::Utc;
use common::{
    calendar::{Calendar, CalendarEvent},
    resources::BattleMode,
};
use core::time::Duration;
use portpicker::pick_unused_port;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
};
use tracing::{error, warn};
use world::sim::FileOpts;

const DEFAULT_WORLD_SEED: u32 = 230;
const CONFIG_DIR: &str = "server_config";
const SETTINGS_FILENAME: &str = "settings.ron";
const WHITELIST_FILENAME: &str = "whitelist.ron";
const BANLIST_FILENAME: &str = "banlist.ron";
const SERVER_DESCRIPTION_FILENAME: &str = "description.ron";
const ADMINS_FILENAME: &str = "admins.ron";

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum ServerBattleMode {
    Global(BattleMode),
    PerPlayer { default: BattleMode },
}

impl Default for ServerBattleMode {
    fn default() -> Self { Self::Global(BattleMode::PvP) }
}

impl ServerBattleMode {
    pub fn allow_choosing(&self) -> bool {
        match self {
            ServerBattleMode::Global { .. } => false,
            ServerBattleMode::PerPlayer { .. } => true,
        }
    }

    pub fn default_mode(&self) -> BattleMode {
        match self {
            ServerBattleMode::Global(mode) => *mode,
            ServerBattleMode::PerPlayer { default: mode } => *mode,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Protocol {
    Quic {
        address: SocketAddr,
        cert_file_path: PathBuf,
        key_file_path: PathBuf,
    },
    Tcp {
        address: SocketAddr,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameplaySettings {
    #[serde(default)]
    pub battle_mode: ServerBattleMode,
    #[serde(default)]
    pub safe_spawn: bool,
    #[serde(default)]
    pub explosion_burn_marks: bool,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            battle_mode: ServerBattleMode::default(),
            safe_spawn: false,
            explosion_burn_marks: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModerationSettings {
    #[serde(default)]
    pub banned_words_files: Vec<PathBuf>,
    #[serde(default)]
    pub automod: bool,
    #[serde(default)]
    pub admins_exempt: bool,
}

impl ModerationSettings {
    pub fn load_banned_words(&self, data_dir: &Path) -> Vec<String> {
        let mut banned_words = Vec::new();
        for fname in self.banned_words_files.iter() {
            let mut path = with_config_dir(data_dir);
            path.push(fname);
            match std::fs::File::open(&path) {
                Ok(file) => match ron::de::from_reader(&file) {
                    Ok(mut words) => banned_words.append(&mut words),
                    Err(error) => error!(?error, ?file, "Couldn't read banned words file"),
                },
                Err(error) => error!(?error, ?path, "Couldn't open banned words file"),
            }
        }
        banned_words
    }
}

impl Default for ModerationSettings {
    fn default() -> Self {
        Self {
            banned_words_files: Vec::new(),
            automod: false,
            admins_exempt: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CalendarMode {
    None,
    Auto,
    Timezone(chrono_tz::Tz),
    Events(Vec<CalendarEvent>),
}

impl Default for CalendarMode {
    fn default() -> Self { Self::Auto }
}

impl CalendarMode {
    pub fn calendar_now(&self) -> Calendar {
        match self {
            CalendarMode::None => Calendar::default(),
            CalendarMode::Auto => Calendar::from_tz(None),
            CalendarMode::Timezone(tz) => Calendar::from_tz(Some(*tz)),
            CalendarMode::Events(events) => Calendar::from_events(events.clone()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub gameserver_protocols: Vec<Protocol>,
    pub metrics_address: SocketAddr,
    pub auth_server_address: Option<String>,
    pub max_players: u16,
    pub world_seed: u32,
    pub server_name: String,
    pub start_time: f64,
    /// Length of a day in minutes.
    pub day_length: f64,
    /// When set to None, loads the default map file (if available); otherwise,
    /// uses the value of the file options to decide how to proceed.
    pub map_file: Option<FileOpts>,
    pub max_view_distance: Option<u32>,
    pub max_player_group_size: u32,
    pub client_timeout: Duration,
    pub spawn_town: Option<String>,
    pub max_player_for_kill_broadcast: Option<usize>,
    pub calendar_mode: CalendarMode,

    /// Experimental feature. No guaranteed forwards-compatibility, may be
    /// removed at *any time* with no migration.
    #[serde(default, skip_serializing)]
    pub experimental_terrain_persistence: bool,

    #[serde(default)]
    pub gameplay: GameplaySettings,
    #[serde(default)]
    pub moderation: ModerationSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gameserver_protocols: vec![
                Protocol::Tcp {
                    address: SocketAddr::from((Ipv6Addr::UNSPECIFIED, 14004)),
                },
                Protocol::Tcp {
                    address: SocketAddr::from((Ipv4Addr::UNSPECIFIED, 14004)),
                },
            ],
            metrics_address: SocketAddr::from((Ipv4Addr::LOCALHOST, 14005)),
            auth_server_address: Some("https://auth.veloren.net".into()),
            world_seed: DEFAULT_WORLD_SEED,
            server_name: "Veloren Server".into(),
            max_players: 100,
            day_length: 30.0,
            start_time: 9.0 * 3600.0,
            map_file: None,
            max_view_distance: Some(65),
            max_player_group_size: 6,
            calendar_mode: CalendarMode::Auto,
            client_timeout: Duration::from_secs(40),
            spawn_town: None,
            max_player_for_kill_broadcast: None,
            experimental_terrain_persistence: false,
            gameplay: GameplaySettings::default(),
            moderation: ModerationSettings::default(),
        }
    }
}

impl Settings {
    /// path: Directory that contains the server config directory
    pub fn load(path: &Path) -> Self {
        let path = Self::get_settings_path(path);

        let mut settings = if let Ok(file) = fs::File::open(&path) {
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
        };

        settings.validate();
        settings
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
        let load = Self::load(path);
        Self {
            // BUG: theoretically another process can grab the port between here and server
            // creation, however the time window is quite small.
            gameserver_protocols: vec![Protocol::Tcp {
                address: SocketAddr::from((
                    Ipv4Addr::LOCALHOST,
                    pick_unused_port().expect("Failed to find unused port!"),
                )),
            }],
            metrics_address: SocketAddr::from((
                Ipv4Addr::LOCALHOST,
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

    fn validate(&mut self) {
        const INVALID_SETTING_MSG: &str =
            "Invalid value for setting in userdata/server/server_config/settings.ron.";

        let default_values = Settings::default();

        if self.day_length <= 0.0 {
            warn!(
                "{} Setting: day_length, Value: {}. Set day_length to it's default value of {}. \
                 Help: day_length must be a positive floating point value above 0.",
                INVALID_SETTING_MSG, self.day_length, default_values.day_length
            );
            self.day_length = default_values.day_length;
        }
    }
}

pub enum InvalidSettingsError {
    InvalidDayDuration,
}
impl Display for InvalidSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidSettingsError::InvalidDayDuration => {
                f.write_str("Invalid settings error: Day length was invalid (zero or negative).")
            },
        }
    }
}

pub fn with_config_dir(path: &Path) -> PathBuf {
    let mut path = PathBuf::from(path);
    path.push(CONFIG_DIR);
    path
}

/// Our upgrade guarantee is that if validation succeeds
/// for an old version, then migration to the next version must always succeed
/// and produce a valid settings file for that version (if we need to change
/// this in the future, it should require careful discussion).  Therefore, we
/// would normally panic if the upgrade produced an invalid settings file, which
/// we would perform by doing the following post-validation (example
/// is given for a hypothetical upgrade from Whitelist_V1 to Whitelist_V2):
///
/// Ok(Whitelist_V2::try_into().expect())
const MIGRATION_UPGRADE_GUARANTEE: &str = "Any valid file of an old verison should be able to \
                                           successfully migrate to the latest version.";

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

        let mut server_description = ServerDescription::default();
        *server_description = "Who needs friends anyway?".into();

        let mut admins = Admins::default();
        // TODO: Let the player choose if they want to use admin commands or not
        admins.insert(
            crate::login_provider::derive_singleplayer_uuid(),
            AdminRecord {
                username_when_admined: Some("singleplayer".into()),
                date: Utc::now(),
                role: admin::Role::Admin,
            },
        );

        Self {
            server_description,
            admins,
            ..load
        }
    }
}
