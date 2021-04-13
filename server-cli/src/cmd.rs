use core::time::Duration;
use server::persistence::SqlLogMode;
use std::sync::mpsc::Sender;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub enum Message {
    AbortShutdown,
    Shutdown { grace_period: Duration },
    Quit,
    AddAdmin(String),
    RemoveAdmin(String),
    LoadArea(u32),
    SetSqlLogMode(SqlLogMode),
    DisconnectAllClients,
}

struct Command<'a> {
    name: &'a str,
    description: &'a str,
    // Whether or not the command splits the arguments on whitespace
    split_spaces: bool,
    args: usize,
    cmd: fn(Vec<String>, &mut Sender<Message>),
}

// TODO: maybe we could be using clap here?
const COMMANDS: [Command; 8] = [
    Command {
        name: "quit",
        description: "Closes the server",
        split_spaces: true,
        args: 0,
        cmd: |_, sender| send(sender, Message::Quit),
    },
    Command {
        name: "shutdown",
        description: "Initiates a graceful shutdown of the server, waiting the specified number \
                      of seconds before shutting down",
        split_spaces: true,
        args: 1,
        cmd: |args, sender| {
            if let Ok(grace_period) = args.first().unwrap().parse::<u64>() {
                send(sender, Message::Shutdown {
                    grace_period: Duration::from_secs(grace_period),
                })
            } else {
                error!("Grace period must an integer")
            }
        },
    },
    Command {
        name: "disconnectall",
        description: "Disconnects all connected clients",
        split_spaces: true,
        args: 0,
        cmd: |_, sender| send(sender, Message::DisconnectAllClients),
    },
    Command {
        name: "loadarea",
        description: "Loads up the chunks in a random area and adds a entity that mimics a player \
                      to keep them from despawning",
        split_spaces: true,
        args: 1,
        cmd: |args, sender| {
            if let Ok(view_distance) = args.first().unwrap().parse::<u32>() {
                send(sender, Message::LoadArea(view_distance));
            } else {
                error!("View distance must be an integer");
            }
        },
    },
    Command {
        name: "abortshutdown",
        description: "Aborts a shutdown if one is in progress",
        split_spaces: false,
        args: 0,
        cmd: |_, sender| send(sender, Message::AbortShutdown),
    },
    Command {
        name: "admin",
        description: "Add or remove an admin via \'admin add/remove <username>\'",
        split_spaces: true,
        args: 2,
        cmd: |args, sender| match args.get(..2) {
            Some([op, username]) if op == "add" => {
                send(sender, Message::AddAdmin(username.clone()));
            },
            Some([op, username]) if op == "remove" => {
                send(sender, Message::RemoveAdmin(username.clone()));
            },
            Some(_) => error!("First arg must be add or remove"),
            _ => error!("Not enough args, should be unreachable"),
        },
    },
    Command {
        name: "sqllog",
        description: "Sets the SQL logging mode, valid values are off, trace and profile",
        split_spaces: true,
        args: 1,
        cmd: |args, sender| match args.get(0) {
            Some(arg) => {
                let sql_log_mode = match arg.to_lowercase().as_str() {
                    "off" => Some(SqlLogMode::Disabled),
                    "profile" => Some(SqlLogMode::Profile),
                    "trace" => Some(SqlLogMode::Trace),
                    _ => None,
                };

                if let Some(sql_log_mode) = sql_log_mode {
                    send(sender, Message::SetSqlLogMode(sql_log_mode));
                } else {
                    error!("Invalid SQL log mode");
                }
            },
            _ => error!("Not enough args"),
        },
    },
    Command {
        name: "help",
        description: "List all command available",
        split_spaces: true,
        args: 0,
        cmd: |_, _| {
            info!("===== Help =====");
            for command in COMMANDS.iter() {
                info!("{} - {}", command.name, command.description)
            }
            info!("================");
        },
    },
];

fn send(sender: &mut Sender<Message>, message: Message) {
    sender
        .send(message)
        .unwrap_or_else(|err| error!("Failed to send CLI message, err: {:?}", err));
}

pub fn parse_command(input: &str, msg_s: &mut Sender<Message>) {
    let mut args = input.split_whitespace();

    if let Some(cmd_name) = args.next() {
        if let Some(cmd) = COMMANDS.iter().find(|cmd| cmd.name == cmd_name) {
            let args = args.collect::<Vec<_>>();

            let (arg_len, args) = if cmd.split_spaces {
                (
                    args.len(),
                    args.into_iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                )
            } else {
                (0, vec![args.into_iter().collect::<String>()])
            };

            use core::cmp::Ordering;
            match arg_len.cmp(&cmd.args) {
                Ordering::Less => error!("{} takes {} arguments", cmd_name, cmd.args),
                Ordering::Greater => {
                    warn!("{} only takes {} arguments", cmd_name, cmd.args);
                    let cmd = cmd.cmd;

                    cmd(args, msg_s)
                },
                Ordering::Equal => {
                    let cmd = cmd.cmd;

                    cmd(args, msg_s)
                },
            }
        } else {
            error!("{} not found", cmd_name);
        }
    }
}
