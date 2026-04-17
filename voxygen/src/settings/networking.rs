use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

/// `NetworkingSettings` stores server and networking settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkingSettings {
    pub username: String,
    pub servers: Vec<String>,
    pub default_server: String,
    pub trusted_auth_servers: HashSet<String>,
    pub use_srv: bool,
    pub use_quic: bool,
    pub validate_tls: bool,
    pub player_physics_behavior: bool,
    pub lossy_terrain_compression: bool,
    pub enable_discord_integration: bool,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            username: "".to_string(),
            servers: vec!["localhost:14004".to_string()],
            default_server: "localhost:14004".to_string(),
            // Nova-Forge does not require authentication; no trusted auth servers needed.
            trusted_auth_servers: HashSet::new(),
            use_srv: true,
            use_quic: false,
            validate_tls: true,
            player_physics_behavior: false,
            lossy_terrain_compression: false,
            enable_discord_integration: false,
        }
    }
}
