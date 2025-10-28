use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};
use tracing::{error, warn};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[expect(clippy::upper_case_acronyms)]
pub enum ShutdownSignal {
    SIGUSR1,
    SIGUSR2,
    SIGTERM,
}

impl ShutdownSignal {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn to_signal(self) -> core::ffi::c_int {
        match self {
            Self::SIGUSR1 => signal_hook::consts::SIGUSR1,
            Self::SIGUSR2 => signal_hook::consts::SIGUSR2,
            Self::SIGTERM => signal_hook::consts::SIGTERM,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub update_shutdown_grace_period_secs: u32,
    pub update_shutdown_message: String,
    pub web_address: SocketAddr,
    /// SECRET API HEADER used to access the chat api, if disabled the API is
    /// unreachable
    pub web_chat_secret: Option<String>,
    /// public SECRET API HEADER used to access the /ui_api, if disabled the API
    /// is reachable localhost only (by /ui)
    pub ui_api_secret: Option<String>,
    pub shutdown_signals: Vec<ShutdownSignal>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            update_shutdown_grace_period_secs: 120,
            update_shutdown_message: "The server is restarting for an update".to_owned(),
            web_address: SocketAddr::from((Ipv4Addr::LOCALHOST, 14005)),
            web_chat_secret: None,
            ui_api_secret: None,
            shutdown_signals: if cfg!(any(target_os = "linux", target_os = "macos")) {
                vec![ShutdownSignal::SIGUSR1]
            } else {
                Vec::new()
            },
        }
    }
}

impl Settings {
    const FILENAME: &str = "settings.ron";

    pub fn load() -> Option<Self> {
        let path = Self::get_settings_path();
        let template_path = path.with_extension("template.ron");

        let settings = if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader(file) {
                Ok(s) => return Some(s),
                Err(e) => {
                    error!(
                        ?e,
                        "FATAL: Failed to parse setting file! Creating a template file for you to \
                         migrate your current settings file: {}",
                        template_path.display()
                    );
                    None
                },
            }
        } else {
            warn!(
                "Settings file not found! Creating a template file: {} — If you wish to change \
                 any settings, copy/move the template to {} and edit the fields as you wish.",
                template_path.display(),
                Self::FILENAME
            );
            Some(Self::default())
        };

        // This is reached if either:
        // - The file can't be opened (presumably it doesn't exist)
        // - Or there was an error parsing the file
        if let Err(e) = Self::save_template(&template_path) {
            error!(?e, "Failed to create template settings file!");
        }

        settings
    }

    fn save_template(path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let ron = ron::ser::to_string_pretty(&Self::default(), ron::ser::PrettyConfig::default())
            .unwrap();
        fs::write(path, ron.as_bytes())
    }

    pub fn get_settings_path() -> PathBuf {
        let mut path = data_dir();
        path.push(Self::FILENAME);
        path
    }
}

pub fn data_dir() -> PathBuf {
    let mut path = common_base::userdata_dir_workspace!();
    path.push("server-cli");
    path
}
