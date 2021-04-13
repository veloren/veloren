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
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            username: "".to_string(),
            servers: vec!["server.veloren.net".to_string()],
            default_server: "server.veloren.net".to_string(),
            trusted_auth_servers: ["https://auth.veloren.net"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}
