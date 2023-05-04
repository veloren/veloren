use clap::{Arg, Command};
use std::{thread, time::Duration};
use tracing::error;

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
    },
}

pub struct Tui {
    _handle: thread::JoinHandle<()>,
}

impl Tui {
    pub fn new() -> (Self, async_channel::Receiver<Cmd>) {
        let (mut commands_s, commands_r) = async_channel::unbounded();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            let mut readline =
                rustyline::Editor::<(), rustyline::history::FileHistory>::with_history(
                    Default::default(),
                    Default::default(),
                )
                .unwrap();
            while let Ok(cmd) = readline.readline("\n\nbotclient> ") {
                let keep_going = Self::process_command(&cmd, &mut commands_s);
                readline.add_history_entry(cmd).unwrap();
                if !keep_going {
                    break;
                }
            }
        });

        (Self { _handle: handle }, commands_r)
    }

    pub fn process_command(cmd: &str, command_s: &mut async_channel::Sender<Cmd>) -> bool {
        let matches = Command::new("veloren-botclient")
            .version(common::util::DISPLAY_VERSION_LONG.as_str())
            .author("The veloren devs <https://gitlab.com/veloren/veloren>")
            .about("The veloren bot client allows logging in as a horde of bots for load-testing")
            .no_binary_name(true)
            .subcommand(
                Command::new("register")
                    .about("Register more bots with the auth server")
                    .args(&[
                        Arg::new("prefix").required(true),
                        Arg::new("password").required(true),
                        Arg::new("count"),
                    ]),
            )
            .subcommand(
                Command::new("login")
                    .about("Login all registered bots whose username starts with a prefix")
                    .args(&[Arg::new("prefix").required(true)]),
            )
            .subcommand(
                Command::new("ingame")
                    .about("Join the world with some random character")
                    .args(&[Arg::new("prefix").required(true)]),
            )
            .try_get_matches_from(cmd.split(' '));
        use clap::error::ErrorKind::*;
        match matches {
            Ok(matches) => {
                if match matches.subcommand() {
                    Some(("register", matches)) => command_s.try_send(Cmd::Register {
                        prefix: matches.get_one::<String>("prefix").unwrap().to_string(),
                        password: matches.get_one::<String>("password").unwrap().to_string(),
                        count: matches.get_one::<usize>("count").cloned(),
                    }),
                    Some(("login", matches)) => command_s.try_send(Cmd::Login {
                        prefix: matches.get_one::<String>("prefix").unwrap().to_string(),
                    }),
                    Some(("ingame", matches)) => command_s.try_send(Cmd::InGame {
                        prefix: matches.get_one::<String>("prefix").unwrap().to_string(),
                    }),
                    _ => Ok(()),
                }
                .is_err()
                {
                    return false;
                }
            },
            Err(e)
                if [DisplayHelp, MissingRequiredArgument, UnknownArgument].contains(&e.kind()) =>
            {
                let _ = e.print();
            },
            Err(e) => {
                error!("{:?}", e);
                return false;
            },
        }
        true
    }
}
