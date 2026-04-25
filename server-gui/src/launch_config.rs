use server::{DEFAULT_WORLD_SEED, persistence::DatabaseSettings};
use std::{net::SocketAddr, path::PathBuf};

/// The values the operator configures on the launch screen before starting
/// the server.  Converted to a `server::Settings` by `build_settings()`.
#[derive(Clone, Debug)]
pub struct LaunchConfig {
    /// Human-readable server name shown to connecting clients.
    pub server_name: String,
    /// World-generation seed.  Changing this after a world already exists has
    /// no effect (the seed is baked into the saved world).
    pub world_seed: u32,
    /// Maximum simultaneous player count.
    pub max_players: u16,
    /// In-game day length in real-world minutes.
    pub day_length: f64,
    /// TCP listen port for game connections.
    pub port: u16,
    /// Enable Track B "Experimental" world-generation lane (Nova-Forge).
    ///
    /// When `false` the stable (upstream) Track A pipeline is used.
    /// When `true` the server boots into the Nova-Forge experimental lane and
    /// a bright banner is shown in the GUI.
    pub experimental_worldgen: bool,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            server_name: "Nova-Forge Server".into(),
            world_seed: DEFAULT_WORLD_SEED,
            max_players: 100,
            day_length: 30.0, // DAY_LENGTH_DEFAULT (minutes)
            port: 14004,
            experimental_worldgen: false,
        }
    }
}

impl LaunchConfig {
    /// Convert the GUI configuration into a `server::Settings` suitable for
    /// passing to `Server::new`.
    pub fn build_settings(&self) -> server::Settings {
        use std::net::{Ipv4Addr, Ipv6Addr};

        let port = self.port;
        let mut s = server::Settings::default();
        s.server_name = self.server_name.clone();
        s.world_seed = self.world_seed;
        s.max_players = self.max_players;
        s.day_length = self.day_length;
        s.experimental_worldgen = self.experimental_worldgen;

        // Replace the default bind addresses with the configured port.
        s.gameserver_protocols = vec![
            server::settings::Protocol::Tcp {
                address: SocketAddr::from((Ipv6Addr::UNSPECIFIED, port)),
            },
            server::settings::Protocol::Tcp {
                address: SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)),
            },
        ];

        s
    }

    /// Build `DatabaseSettings` relative to a data directory.
    pub fn build_db_settings(&self, server_data_dir: &std::path::Path) -> DatabaseSettings {
        DatabaseSettings {
            db_dir: server_data_dir.join("saves"),
            sql_log_mode: server::persistence::SqlLogMode::Disabled,
        }
    }
}

/// Editable string buffer for the seed field (hex or decimal input).
#[derive(Clone, Default)]
pub struct SeedInput {
    pub text: String,
    pub error: bool,
}

impl SeedInput {
    pub fn from_u32(v: u32) -> Self {
        Self {
            text: v.to_string(),
            error: false,
        }
    }

    /// Try to parse as decimal or `0x`-prefixed hex.  Returns the parsed value
    /// on success and sets `error` on failure.
    pub fn parse(&mut self) -> Option<u32> {
        let t = self.text.trim();
        let result = if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            t.parse::<u32>().ok()
        };
        self.error = result.is_none();
        result
    }
}
