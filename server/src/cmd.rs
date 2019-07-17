//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to `CHAT_COMMANDS`
//! and provide a handler function.

use crate::Server;
use common::{
    comp,
    msg::ServerMsg,
    npc::{get_npc_name, NpcKind},
    state::TimeOfDay,
};
use specs::{Builder, Entity as EcsEntity, Join};
use vek::*;

use lazy_static::lazy_static;
use scan_fmt::scan_fmt;
/// Struct representing a command that a user can run from server chat.
pub struct ChatCommand {
    /// The keyword used to invoke the command, omitting the leading '/'.
    pub keyword: &'static str,
    /// A format string for parsing arguments.
    arg_fmt: &'static str,
    /// A message that explains how the command is used.
    help_string: &'static str,
    /// Handler function called when the command is executed.
    /// # Arguments
    /// * `&mut Server` - the `Server` instance executing the command.
    /// * `EcsEntity` - an `Entity` corresponding to the player that invoked the command.
    /// * `String` - a `String` containing the part of the command after the keyword.
    /// * `&ChatCommand` - the command to execute with the above arguments.
    /// Handler functions must parse arguments from the the given `String` (`scan_fmt!` is included for this purpose).
    handler: fn(&mut Server, EcsEntity, String, &ChatCommand),
}

impl ChatCommand {
    /// Creates a new chat command.
    pub fn new(
        keyword: &'static str,
        arg_fmt: &'static str,
        help_string: &'static str,
        handler: fn(&mut Server, EcsEntity, String, &ChatCommand),
    ) -> Self {
        Self {
            keyword,
            arg_fmt,
            help_string,
            handler,
        }
    }
    /// Calls the contained handler function, passing `&self` as the last argument.
    pub fn execute(&self, server: &mut Server, entity: EcsEntity, args: String) {
        (self.handler)(server, entity, args, self);
    }
}

lazy_static! {
    /// Static list of chat commands available to the server.
    pub static ref CHAT_COMMANDS: Vec<ChatCommand> = vec![
        ChatCommand::new(
            "jump",
            "{d} {d} {d}",
            "/jump <dx> <dy> <dz> : Offset your current position",
            handle_jump,
        ),
        ChatCommand::new(
            "goto",
            "{d} {d} {d}",
            "/goto <x> <y> <z> : Teleport to a position",
            handle_goto,
        ),
        ChatCommand::new(
            "alias",
            "{}",
            "/alias <name> : Change your alias",
            handle_alias,
        ),
        ChatCommand::new(
            "tp",
            "{}",
            "/tp <alias> : Teleport to another player",
            handle_tp,
        ),
        ChatCommand::new(
            "kill",
            "{}",
            "/kill : Kill yourself",
            handle_kill,
        ),
        ChatCommand::new(
            "time",
            "{} {s}",
            "/time : Set the time of day",
            handle_time,
        ),
        ChatCommand::new(
            "spawn",
            "{} {} {d}",
            "/spawn <alignment> <entity> [amount] : Spawn a test entity",
            handle_spawn,
        ),
        ChatCommand::new(
             "players",
             "{}",
             "/players : Show the online players list",
             handle_players,
         ),
        ChatCommand::new(
            "help", "", "/help: Display this message", handle_help),
        ChatCommand::new(
            "health",
            "{}",
            "/health : Set your current health",
            handle_health,
        ),
        ChatCommand::new(
            "build",
            "",
            "/build : Toggles build mode on and off",
            handle_build,
        ),
        ChatCommand::new(
            "msg",
            "{}",
            "/msg <alias> : Send a message to another player",
            handle_msg,
        ),
        ChatCommand::new(
            "killnpcs",
            "{}",
            "/killnpcs : Kill the NPCs",
            handle_killnpcs,
         ),
    ];
}

fn handle_jump(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_x, opt_y, opt_z) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32);
    match (opt_x, opt_y, opt_z) {
        (Some(x), Some(y), Some(z)) => {
            match server.state.read_component_cloned::<comp::Pos>(entity) {
                Some(current_pos) => {
                    server
                        .state
                        .write_component(entity, comp::Pos(current_pos.0 + Vec3::new(x, y, z)));
                    server.state.write_component(entity, comp::ForceUpdate);
                }
                None => server.clients.notify(
                    entity,
                    ServerMsg::Chat(String::from("You have no position!")),
                ),
            }
        }
        _ => server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
    }
}

fn handle_goto(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_x, opt_y, opt_z) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32);
    match server.state.read_component_cloned::<comp::Pos>(entity) {
        Some(_pos) => match (opt_x, opt_y, opt_z) {
            (Some(x), Some(y), Some(z)) => {
                server
                    .state
                    .write_component(entity, comp::Pos(Vec3::new(x, y, z)));
                server.state.write_component(entity, comp::ForceUpdate);
            }
            _ => server
                .clients
                .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
        },
        None => {
            server.clients.notify(
                entity,
                ServerMsg::Chat(String::from("You don't have any position!")),
            );
        }
    }
}

fn handle_kill(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(entity)
        .map(|s| s.health.set_to(0, comp::HealthSource::Suicide));
}

fn handle_time(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let time = scan_fmt!(&args, action.arg_fmt, String);
    server.state.ecs_mut().write_resource::<TimeOfDay>().0 = match time.as_ref().map(|s| s.as_str())
    {
        Some("day") => 12.0 * 3600.0,
        Some("night") => 24.0 * 3600.0,
        Some("dawn") => 5.0 * 3600.0,
        Some("dusk") => 17.0 * 3600.0,
        Some(n) => match n.parse() {
            Ok(n) => n,
            Err(_) => {
                server
                    .clients
                    .notify(entity, ServerMsg::Chat(format!("'{}' is not a time!", n)));
                return;
            }
        },
        None => {
            server.clients.notify(
                entity,
                ServerMsg::Chat("You must specify a time!".to_string()),
            );
            return;
        }
    };
}

fn handle_health(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_hp = scan_fmt!(&args, action.arg_fmt, u32);

    match server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(entity)
    {
        Some(stats) => match opt_hp {
            Some(hp) => stats.health.set_to(hp, comp::HealthSource::Command),
            None => {
                server.clients.notify(
                    entity,
                    ServerMsg::Chat(String::from("You must specify health amount!")),
                );
            }
        },
        None => server.clients.notify(
            entity,
            ServerMsg::Chat(String::from("You have no position.")),
        ),
    }
}

fn handle_alias(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_alias = scan_fmt!(&args, action.arg_fmt, String);
    match opt_alias {
        Some(alias) => {
            server
                .state
                .ecs_mut()
                .write_storage::<comp::Player>()
                .get_mut(entity)
                .map(|player| player.alias = alias);
        }
        None => server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
    }
}

fn handle_tp(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_alias = scan_fmt!(&args, action.arg_fmt, String);
    match opt_alias {
        Some(alias) => {
            let ecs = server.state.ecs();
            let opt_player = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
                .join()
                .find(|(_, player)| player.alias == alias)
                .map(|(entity, _)| entity);
            match server.state.read_component_cloned::<comp::Pos>(entity) {
                Some(_pos) => match opt_player {
                    Some(player) => match server.state.read_component_cloned::<comp::Pos>(player) {
                        Some(pos) => {
                            server.state.write_component(entity, pos);
                            server.state.write_component(entity, comp::ForceUpdate);
                        }
                        None => server.clients.notify(
                            entity,
                            ServerMsg::Chat(format!("Unable to teleport to player '{}'!", alias)),
                        ),
                    },
                    None => {
                        server.clients.notify(
                            entity,
                            ServerMsg::Chat(format!("Player '{}' not found!", alias)),
                        );
                        server
                            .clients
                            .notify(entity, ServerMsg::Chat(String::from(action.help_string)));
                    }
                },
                None => {
                    server
                        .clients
                        .notify(entity, ServerMsg::Chat(format!("You have no position!")));
                }
            }
        }
        None => server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
    }
}

fn handle_spawn(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_align, opt_id, opt_amount) = scan_fmt!(&args, action.arg_fmt, String, NpcKind, String);
    // This should be just an enum handled with scan_fmt!
    let opt_agent = alignment_to_agent(&opt_align.unwrap_or(String::new()), entity);

    // Make sure the amount is either not provided or a valid value
    let opt_amount = opt_amount
        .map_or(Some(1), |a| a.parse().ok())
        .and_then(|a| if a > 0 { Some(a) } else { None });

    match (opt_agent, opt_id, opt_amount) {
        (Some(agent), Some(id), Some(amount)) => {
            match server.state.read_component_cloned::<comp::Pos>(entity) {
                Some(mut pos) => {
                    pos.0.x += 1.0; // Temp fix TODO: Solve NaN issue with positions of pets
                    for _ in 0..amount {
                        let body = kind_to_body(id);
                        server
                            .create_npc(pos, get_npc_name(id), body)
                            .with(agent)
                            .build();
                    }
                    server.clients.notify(
                        entity,
                        ServerMsg::Chat(format!("Spawned {} entities", amount).to_owned()),
                    );
                }
                None => server
                    .clients
                    .notify(entity, ServerMsg::Chat("You have no position!".to_owned())),
            }
        }
        _ => server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
    }
}

fn handle_players(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    let ecs = server.state.ecs();
    let players = ecs.read_storage::<comp::Player>();
    let count = players.join().count();
    let header_message: String = format!("{} online players: \n", count);
    if count > 0 {
        let mut player_iter = players.join();
        let first = player_iter.next().unwrap().alias.to_owned();
        let player_list = player_iter.fold(first, |mut s, p| {
            s += ",\n";
            s += &p.alias;
            s
        });

        server
            .clients
            .notify(entity, ServerMsg::Chat(header_message + &player_list));
    } else {
        server
            .clients
            .notify(entity, ServerMsg::Chat(header_message));
    }
}

fn handle_build(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    if server
        .state
        .read_storage::<comp::CanBuild>()
        .get(entity)
        .is_some()
    {
        server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .remove(entity);
        server.clients.notify(
            entity,
            ServerMsg::Chat(String::from("Toggled off build mode!")),
        );
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .insert(entity, comp::CanBuild);
        server.clients.notify(
            entity,
            ServerMsg::Chat(String::from("Toggled on build mode!")),
        );
    }
}

fn handle_help(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    for cmd in CHAT_COMMANDS.iter() {
        server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(cmd.help_string)));
    }
}

fn alignment_to_agent(alignment: &str, target: EcsEntity) -> Option<comp::Agent> {
    match alignment {
        "hostile" => Some(comp::Agent::Enemy { target: None }),
        "friendly" => Some(comp::Agent::Pet {
            target,
            offset: Vec2::zero(),
        }),
        // passive?
        _ => None,
    }
}

fn kind_to_body(kind: NpcKind) -> comp::Body {
    match kind {
        NpcKind::Humanoid => comp::Body::Humanoid(comp::humanoid::Body::random()),
        NpcKind::Pig => comp::Body::Quadruped(comp::quadruped::Body::random()),
        NpcKind::Wolf => comp::Body::QuadrupedMedium(comp::quadruped_medium::Body::random()),
    }
}

fn handle_killnpcs(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    let ecs = server.state.ecs();
    let mut stats = ecs.write_storage::<comp::Stats>();
    let players = ecs.read_storage::<comp::Player>();
    let mut count = 0;
    for (stats, ()) in (&mut stats, !&players).join() {
        count += 1;
        stats.health.set_to(0, comp::HealthSource::Command);
    }
    let text = if count > 0 {
        format!("Destroyed {} NPCs.", count)
    } else {
        "No NPCs on server.".to_string()
    };
    server.clients.notify(entity, ServerMsg::Chat(text));
}

fn handle_msg(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_alias = scan_fmt!(&args, action.arg_fmt, String);
    match opt_alias {
        Some(alias) => {
            let ecs = server.state.ecs();
            let opt_player = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
                .join()
                .find(|(_, player)| player.alias == alias)
                .map(|(entity, _)| entity);
            let msg = &args[alias.len()..args.len()];
            match opt_player {
                Some(player) => {
                    if msg.len() > 1 {
                        let opt_name = ecs
                            .read_storage::<comp::Player>()
                            .get(entity)
                            .map(|s| s.alias.clone());
                        match opt_name {
                            Some(name) => {
                                server.clients.notify(
                                    player,
                                    ServerMsg::Chat(format!("{} tells you:{}", name, msg)),
                                );
                            }
                            None => {
                                server.clients.notify(
                                    entity,
                                    ServerMsg::Chat(String::from("You do not exist!")),
                                );
                            }
                        }
                    } else {
                        server.clients.notify(
                            entity,
                            ServerMsg::Chat(format!(
                                "You really should say something to {}!",
                                alias
                            )),
                        );
                    }
                }
                None => {
                    server.clients.notify(
                        entity,
                        ServerMsg::Chat(format!("Player '{}' not found!", alias)),
                    );
                }
            }
        }
        None => server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
    }
}

