use clap::{App, AppSettings, Arg, SubCommand};
use std::{thread, time::Duration};
use tracing::error;

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

pub enum Cmd {
    Register {
        prefix: String,
        password: String,
        count: Option<usize>,
    },
    Login {
        prefix: String,
    },
    InGame {
        prefix: String,
    }
}

pub struct Tui {
    _handle: thread::JoinHandle<()>,
}

impl Tui {
    pub fn new() -> (Self, async_channel::Receiver<Cmd>) {
        let (mut commands_s, commands_r) = async_channel::unbounded();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            let mut readline = rustyline::Editor::<()>::new();
            loop {
                match readline.readline("\n\nbotclient> ") {
                    Ok(cmd) => {
                        let keep_going = Self::process_command(&cmd, &mut commands_s);
                        readline.add_history_entry(cmd);
                        if !keep_going {
                            break;
                        }
                    },
                    Err(_) => break,
                }
            }
        });

        (Self { _handle: handle }, commands_r)
    }

    pub fn process_command(cmd: &str, command_s: &mut async_channel::Sender<Cmd>) -> bool {
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
            .subcommand(
                SubCommand::with_name("ingame")
                    .about("Join the world with some random character")
                    .args(&[Arg::with_name("prefix").required(true)]),
            )
            .get_matches_from_safe(cmd.split(" "));
        use clap::ErrorKind::*;
        match matches {
            Ok(matches) => {
                if match matches.subcommand() {
                    ("register", Some(matches)) => command_s.try_send(Cmd::Register {
                        prefix: matches.value_of("prefix").unwrap().to_string(),
                        password: matches.value_of("password").unwrap().to_string(),
                        count: matches
                            .value_of("count")
                            .and_then(|x| x.parse::<usize>().ok()),
                    }),
                    ("login", Some(matches)) => command_s.try_send(Cmd::Login {
                        prefix: matches.value_of("prefix").unwrap().to_string(),
                    }),
                    ("ingame", Some(matches)) => command_s.try_send(Cmd::InGame {
                        prefix: matches.value_of("prefix").unwrap().to_string(),
                    }),
                    _ => Ok(()),
                }
                .is_err()
                {
                    return false;
                }
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
}
