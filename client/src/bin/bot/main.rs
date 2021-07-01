#[macro_use] extern crate serde;

use authc::AuthClient;
use common::{clock::Clock, comp};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::runtime::Runtime;
use tracing::{info, trace, warn};
use veloren_client::{addr::ConnectionArgs, Client};

mod settings;
mod tui;

use common::comp::body::humanoid::Body;
use settings::Settings;
use tui::Cmd;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BotCreds {
    username: String,
    password: String,
}

pub fn main() {
    common_frontend::init_stdout(None);

    let settings = Settings::load();
    info!("Settings: {:?}", settings);

    let (_tui, cmds) = tui::Tui::new();
    let mut bc = BotClient::new(settings);
    'outer: loop {
        loop {
            match cmds.try_recv() {
                Ok(cmd) => bc.cmd(cmd),
                Err(async_channel::TryRecvError::Empty) => break,
                Err(async_channel::TryRecvError::Closed) => break 'outer,
            }
        }
        bc.tick();
    }
    info!("shutdown complete");
}

pub struct BotClient {
    settings: Settings,
    runtime: Arc<Runtime>,
    menu_client: Client,
    bot_clients: HashMap<String, Client>,
    clock: Clock,
}

pub fn make_client(runtime: &Arc<Runtime>, server: &str) -> Client {
    let runtime_clone = Arc::clone(&runtime);
    let addr = ConnectionArgs::Tcp {
        prefer_ipv6: false,
        hostname: server.to_owned(),
    };
    runtime
        .block_on(Client::new(addr, runtime_clone, &mut None))
        .expect("Failed to connect to server")
}

impl BotClient {
    pub fn new(settings: Settings) -> BotClient {
        let runtime = Arc::new(Runtime::new().unwrap());
        let menu_client: Client = make_client(&runtime, &settings.server);
        let clock = Clock::new(Duration::from_secs_f64(1.0 / 60.0));
        BotClient {
            settings,
            runtime,
            menu_client,
            bot_clients: HashMap::new(),
            clock,
        }
    }

    pub fn tick(&mut self) {
        self.clock.tick();
        for (username, client) in self.bot_clients.iter_mut() {
            trace!(?username, "tick");
            let _msgs: Result<Vec<veloren_client::Event>, veloren_client::Error> =
                client.tick(comp::ControllerInputs::default(), self.clock.dt(), |_| {});
        }
    }

    pub fn cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Register {
                prefix,
                password,
                count,
            } => self.handle_register(&prefix, &password, count),
            Cmd::Login { prefix } => self.handle_login(&prefix),
            Cmd::InGame { prefix } => self.handle_ingame_join(&prefix),
        }
    }

    pub fn handle_register(&mut self, prefix: &str, password: &str, count: Option<usize>) {
        let usernames = match count {
            Some(n) => (0..n)
                .into_iter()
                .map(|i| format!("{}{:03}", prefix, i))
                .collect::<Vec<String>>(),
            None => vec![prefix.to_string()],
        };
        info!("usernames: {:?}", usernames);
        if let Some(auth_addr) = self.menu_client.server_info().auth_provider.as_ref() {
            let (scheme, authority) = auth_addr.split_once("://").expect("invalid auth url");
            let scheme = scheme
                .parse::<authc::Scheme>()
                .expect("invalid auth url scheme");
            let authority = authority
                .parse::<authc::Authority>()
                .expect("invalid auth url authority");

            let authc = AuthClient::new(scheme, authority).expect("couldn't connect to , insecure");
            for username in usernames.iter() {
                if self
                    .settings
                    .bot_logins
                    .iter()
                    .any(|x| *x.username == *username)
                {
                    continue;
                }
                match self.runtime.block_on(authc.register(username, password)) {
                    Ok(()) => {
                        self.settings.bot_logins.push(BotCreds {
                            username: username.to_string(),
                            password: password.to_string(),
                        });
                        self.settings.save_to_file_warn();
                    },
                    Err(e) => {
                        warn!("error registering {:?}: {:?}", username, e);
                        break;
                    },
                }
            }
        } else {
            warn!("Server's auth_provider is None");
        }
        info!("register done");
    }

    pub fn handle_login(&mut self, prefix: &str) {
        let creds: Vec<_> = self
            .settings
            .bot_logins
            .iter()
            .filter(|x| x.username.starts_with(prefix))
            .cloned()
            .collect();
        for cred in creds.iter() {
            let runtime = Arc::clone(&self.runtime);

            let server = self.settings.server.clone();
            let client = self
                .bot_clients
                .entry(cred.username.clone())
                .or_insert_with(|| make_client(&runtime, &server));

            // TODO: log the clients in in parallel instead of in series
            if let Err(e) = runtime.block_on(client.register(
                cred.username.clone(),
                cred.password.clone(),
                |_| true,
            )) {
                warn!("error logging in {:?}: {:?}", cred.username, e);
            }
            let body = BotClient::create_default_body();
            client.create_character(
                cred.username.clone(),
                Some("common.items.weapons.sword.starter".to_string()),
                body.into(),
            );
            client.load_character_list();
        }
        info!("login done");
    }

    fn create_default_body() -> Body {
        comp::body::humanoid::Body {
            species: comp::body::humanoid::Species::Human,
            body_type: comp::body::humanoid::BodyType::Male,
            hair_style: 0,
            beard: 0,
            eyes: 0,
            accessory: 0,
            hair_color: 0,
            skin: 0,
            eye_color: 0,
        }
    }

    pub fn handle_ingame_join(&mut self, prefix: &str) {
        let creds: Vec<_> = self
            .settings
            .bot_logins
            .iter()
            .filter(|x| x.username.starts_with(prefix))
            .cloned()
            .collect();
        for cred in creds.iter() {
            let client = match self.bot_clients.get_mut(&cred.username) {
                Some(c) => c,
                None => {
                    tracing::trace!(?cred.username, "skip not logged in client");
                    continue;
                },
            };

            let list = client.character_list();
            if list.loading || list.characters.is_empty() {
                tracing::trace!(?cred.username, "skip client as it has no character");
                continue;
            }

            let c = list.characters.get(0).unwrap();
            if let Some(id) = c.character.id {
                client.request_character(id);
            }
        }
        info!("ingame done");
    }
}
