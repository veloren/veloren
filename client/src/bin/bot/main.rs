#![feature(str_split_once)]

#[macro_use] extern crate serde;

use authc::AuthClient;
use clap::{App, AppSettings, Arg, SubCommand};
use common::{clock::Clock, comp};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::runtime::Runtime;
use tracing::{error, info, warn};
use veloren_client::{addr::ConnectionArgs, Client};

mod settings;

use settings::Settings;

pub fn init_logging() {
    use termcolor::{ColorChoice, StandardStream};
    use tracing::Level;
    use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};
    const RUST_LOG_ENV: &str = "RUST_LOG";
    let filter = EnvFilter::from_env(RUST_LOG_ENV).add_directive(LevelFilter::INFO.into());
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_env_filter(filter);

    subscriber
        .with_writer(|| StandardStream::stdout(ColorChoice::Auto))
        .init();
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BotCreds {
    username: String,
    password: String,
}

pub fn main() {
    init_logging();

    let settings = Settings::load();
    info!("Settings: {:?}", settings);

    let mut bc = BotClient::new(settings);
    bc.repl();
}

pub struct BotClient {
    settings: Settings,
    readline: rustyline::Editor<()>,
    runtime: Arc<Runtime>,
    menu_client: Client,
    bot_clients: HashMap<String, Client>,
    clock: Clock,
}

pub fn make_client(runtime: &Arc<Runtime>, server: &str) -> Client {
    let runtime2 = Arc::clone(&runtime);
    let view_distance: Option<u32> = None;
    runtime.block_on(async {
        let connection_args = ConnectionArgs::resolve(server, false)
            .await
            .expect("DNS resolution failed");
        Client::new(connection_args, view_distance, runtime2)
            .await
            .expect("Failed to connect to server")
    })
}

impl BotClient {
    pub fn new(settings: Settings) -> BotClient {
        let readline = rustyline::Editor::<()>::new();
        let runtime = Arc::new(Runtime::new().unwrap());
        let menu_client: Client = make_client(&runtime, &settings.server);
        let clock = Clock::new(Duration::from_secs_f64(1.0 / 60.0));
        BotClient {
            settings,
            readline,
            runtime,
            menu_client,
            bot_clients: HashMap::new(),
            clock,
        }
    }

    pub fn repl(&mut self) {
        loop {
            match self.readline.readline("\n\nbotclient> ") {
                Ok(cmd) => {
                    let keep_going = self.process_command(&cmd);
                    self.readline.add_history_entry(cmd);
                    if !keep_going {
                        break;
                    }
                },
                Err(_) => break,
            }
        }
    }

    pub fn process_command(&mut self, cmd: &str) -> bool {
        let matches = App::new("veloren-botclient")
            .version(common::util::DISPLAY_VERSION_LONG.as_str())
            .author("The veloren devs <https://gitlab.com/veloren/veloren>")
            .about("The veloren bot client allows logging in as a horde of bots for load-testing")
            .setting(AppSettings::NoBinaryName)
            .subcommand(
                SubCommand::with_name("register")
                    .about("Register more bots with the auth server")
                    .args(&[
                        Arg::with_name("prefix").required(true),
                        Arg::with_name("password").required(true),
                        Arg::with_name("count"),
                    ]),
            )
            .subcommand(
                SubCommand::with_name("login")
                    .about("Login all registered bots whose username starts with a prefix")
                    .args(&[Arg::with_name("prefix").required(true)]),
            )
            .subcommand(SubCommand::with_name("tick").about("Handle ticks for all logged in bots"))
            .get_matches_from_safe(cmd.split(" "));
        use clap::ErrorKind::*;
        match matches {
            Ok(matches) => match matches.subcommand() {
                ("register", Some(matches)) => self.handle_register(
                    matches.value_of("prefix").unwrap(),
                    matches.value_of("password").unwrap(),
                    matches
                        .value_of("count")
                        .and_then(|x| x.parse::<usize>().ok()),
                ),
                ("login", Some(matches)) => self.handle_login(matches.value_of("prefix").unwrap()),
                ("tick", _) => self.handle_tick(),
                _ => {},
            },
            Err(e)
                if [HelpDisplayed, MissingRequiredArgument, UnknownArgument].contains(&e.kind) =>
            {
                println!("{}", e.message);
            }
            Err(e) => {
                error!("{:?}", e);
                return false;
            },
        }
        true
    }

    pub fn handle_register(&mut self, prefix: &str, password: &str, count: Option<usize>) {
        let usernames = match count {
            Some(n) => (0..n)
                .into_iter()
                .map(|i| format!("{}{}", prefix, i))
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
                    .any(|x| &*x.username == &*username)
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
    }

    pub fn client_for_bot(&mut self, username: &str) -> &mut Client {
        let runtime = Arc::clone(&self.runtime);
        let server = self.settings.server.clone();
        self.bot_clients
            .entry(username.to_string())
            .or_insert_with(|| make_client(&runtime, &server))
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
            let client = self.client_for_bot(&cred.username);
            // TODO: log the clients in in parallel instead of in series
            if let Err(e) = runtime.block_on(client.register(
                cred.username.clone(),
                cred.password.clone(),
                |_| true,
            )) {
                warn!("error logging in {:?}: {:?}", cred.username, e);
            }
            /*let body = comp::body::biped_large::Body {
                species: comp::body::biped_large::Species::Dullahan,
                body_type: comp::body::biped_large::BodyType::Male,
            };*/
            let body = comp::body::humanoid::Body {
                species: comp::body::humanoid::Species::Human,
                body_type: comp::body::humanoid::BodyType::Male,
                hair_style: 0,
                beard: 0,
                eyes: 0,
                accessory: 0,
                hair_color: 0,
                skin: 0,
                eye_color: 0,
            };
            client.create_character(
                cred.username.clone(),
                Some("common.items.weapons.sword.starter".to_string()),
                body.into(),
            );
            //client.create_character(cred.username.clone(),
            // Some("common.items.debug.admin_stick".to_string()), body.into());
        }
    }

    // TODO: maybe do this automatically in a threadpool instead of as a command
    pub fn handle_tick(&mut self) {
        self.clock.tick();
        for (username, client) in self.bot_clients.iter_mut() {
            info!("cl {:?}: {:?}", username, client.character_list());
            let msgs: Result<Vec<veloren_client::Event>, veloren_client::Error> =
                client.tick(comp::ControllerInputs::default(), self.clock.dt(), |_| {});
            info!(
                "msgs {:?}: {:?} {:?}",
                username,
                msgs,
                client.character_list()
            );
        }
    }
}
