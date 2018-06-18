// Standard
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

// Library
use toml;

const CONFIG_FILE: &str = "server_conf.toml";

// Partial config

#[derive(Deserialize)]
pub struct PartialConfig {
    pub network: Option<PartialConfigNetwork>
}
#[derive(Deserialize)]
pub struct PartialConfigNetwork {
    pub port: Option<u16>
}

// Config

#[derive(Serialize)]
pub struct Config {
    pub network: ConfigNetwork
}
#[derive(Serialize)]
pub struct ConfigNetwork {
    pub port: u16
}

impl Config {
    pub fn from_partial(partial: &PartialConfig) -> Config {
        let default = default_config();
        Config {
            network: match &partial.network {
                Some(partial_network) => ConfigNetwork::from_partial(partial_network),
                None => default.network
            }
        }
    }

    pub fn write_to(&self, config_path: &Path) {
        let mut file = File::create(config_path).unwrap();
        file.write_all(&toml::to_string(&default_config()).unwrap().as_bytes());
    }
}
impl ConfigNetwork {
    pub fn from_partial(partial: &PartialConfigNetwork) -> ConfigNetwork {
        let default = default_config();
        ConfigNetwork {
            port: partial.port.unwrap_or(default.network.port),
        }
    }
}

fn default_config() -> Config {
    Config {
        network: ConfigNetwork {
            port: 59003
        }
    }
}

pub fn load_config() -> Config {

    let config_path = Path::new(CONFIG_FILE);

    let config = match File::open(config_path) {
        Ok(mut file) => {
            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => Config::from_partial(&toml::from_str::<PartialConfig>(&contents).unwrap()),
                Err(_) => default_config(),
            }
        }
        Err(_) => default_config(),
    };

    config.write_to(config_path);
    config
}
