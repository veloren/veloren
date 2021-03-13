use core::time::Duration;
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
const COMMANDS: [Command; 6] = [
    Command {
        name: "quit",
        description: "Closes the server",
        split_spaces: true,
        args: 0,
        cmd: |_, sender| sender.send(Message::Quit).unwrap(),
    },
    Command {
        name: "shutdown",
        description: "Initiates a graceful shutdown of the server, waiting the specified number \
                      of seconds before shutting down",
        split_spaces: true,
        args: 1,
        cmd: |args, sender| {
            if let Ok(grace_period) = args.first().unwrap().parse::<u64>() {
                sender
                    .send(Message::Shutdown {
                        grace_period: Duration::from_secs(grace_period),
                    })
                    .unwrap()
            } else {
                error!("Grace period must an integer")
            }
        },
    },
    Command {
        name: "loadarea",
        description: "Loads up the chunks in a random area and adds a entity that mimics a player \
                      to keep them from despawning",
        split_spaces: true,
        args: 1,
        cmd: |args, sender| {
            if let Ok(view_distance) = args.first().unwrap().parse::<u32>() {
                sender.send(Message::LoadArea(view_distance)).unwrap();
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
        cmd: |_, sender| sender.send(Message::AbortShutdown).unwrap(),
    },
    Command {
        name: "admin",
        description: "Add or remove an admin via \'admin add/remove <username>\'",
        split_spaces: true,
        args: 2,
        cmd: |args, sender| match args.get(..2) {
            Some([op, username]) if op == "add" => {
                sender.send(Message::AddAdmin(username.clone())).unwrap()
            },
            Some([op, username]) if op == "remove" => {
                sender.send(Message::RemoveAdmin(username.clone())).unwrap()
            },
            Some(_) => error!("First arg must be add or remove"),
            _ => error!("Not enough args, should be unreachable"),
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
