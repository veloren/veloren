//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to `CHAT_COMMANDS`
//! and provide a handler function.

use crate::Server;
use common::{comp, msg::ServerMsg};
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
            handle_jump
        ),
        ChatCommand::new(
            "goto",
            "{d} {d} {d}",
            "/goto <x> <y> <z> : Teleport to a position",
            handle_goto
        ),
        ChatCommand::new(
            "alias",
            "{}",
            "/alias <name> : Change your alias",
            handle_alias
        ),
        ChatCommand::new(
            "tp",
            "{}",
            "/tp <alias> : Teleport to another player",
            handle_tp
        ),
        ChatCommand::new(
            "kill",
            "{}",
            "/kill : Kill yourself",
            handle_kill
        ),
        ChatCommand::new(
            "pig",
            "{}",
            "/pig : Spawn a test pig NPC",
            handle_petpig
        ),
        ChatCommand::new(
            "wolf",
            "{}",
            "/wolf : Spawn a test wolf NPC",
            handle_petwolf
        ),
        ChatCommand::new(
            "help", "", "/help: Display this message", handle_help)
    ];
}

fn handle_jump(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_x, opt_y, opt_z) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32);
    match (opt_x, opt_y, opt_z) {
        (Some(x), Some(y), Some(z)) => {
            match server
                .state
                .read_component_cloned::<comp::phys::Pos>(entity)
            {
                Some(current_pos) => {
                    server.state.write_component(
                        entity,
                        comp::phys::Pos(current_pos.0 + Vec3::new(x, y, z)),
                    );
                    server
                        .state
                        .write_component(entity, comp::phys::ForceUpdate);
                }
                None => server.clients.notify(
                    entity,
                    ServerMsg::Chat(String::from("Command 'jump' invalid in current state.")),
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
    match (opt_x, opt_y, opt_z) {
        (Some(x), Some(y), Some(z)) => {
            server
                .state
                .write_component(entity, comp::phys::Pos(Vec3::new(x, y, z)));
            server
                .state
                .write_component(entity, comp::phys::ForceUpdate);
        }
        _ => server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
    }
}

fn handle_kill(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(entity)
        .map(|s| s.hp.current = 0);
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
            let opt_player = (&ecs.entities(), &ecs.read_storage::<comp::player::Player>())
                .join()
                .find(|(_, player)| player.alias == alias)
                .map(|(entity, _)| entity);
            match opt_player {
                Some(player) => match server
                    .state
                    .read_component_cloned::<comp::phys::Pos>(player)
                {
                    Some(pos) => {
                        server.state.write_component(entity, pos);
                        server
                            .state
                            .write_component(entity, comp::phys::ForceUpdate);
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
            }
        }
        None => server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
    }
}

fn handle_petpig(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    match server
        .state
        .read_component_cloned::<comp::phys::Pos>(entity)
    {
        Some(mut pos) => {
            pos.0.x += 1.0; // Temp fix TODO: Solve NaN issue with positions of pets
            server
                .create_npc(
                    pos,
                    "Bungo".to_owned(),
                    comp::Body::Quadruped(comp::QuadrupedBody::random()),
                )
                .with(comp::Agent::Pet {
                    target: entity,
                    offset: Vec2::zero(),
                })
                .build();
            server
                .clients
                .notify(entity, ServerMsg::Chat("Spawned pet!".to_owned()));
        }
        None => server
            .clients
            .notify(entity, ServerMsg::Chat("You have no position!".to_owned())),
    }
}
fn handle_petwolf(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    match server
        .state
        .read_component_cloned::<comp::phys::Pos>(entity)
    {
        Some(mut pos) => {
            pos.0.x += 1.0; // Temp fix TODO: Solve NaN issue with positions of pets
            server
                .create_npc(
                    pos,
                    "Tobermory".to_owned(),
                    comp::Body::QuadrupedMedium(comp::QuadrupedMediumBody::random()),
                )
                .with(comp::Agent::Pet {
                    target: entity,
                    offset: Vec2::zero(),
                })
                .build();
            server
                .clients
                .notify(entity, ServerMsg::Chat("Spawned pet!".to_owned()));
        }
        None => server
            .clients
            .notify(entity, ServerMsg::Chat("You have no position!".to_owned())),
    }
}
fn handle_help(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    for cmd in CHAT_COMMANDS.iter() {
        server
            .clients
            .notify(entity, ServerMsg::Chat(String::from(cmd.help_string)));
    }
}
