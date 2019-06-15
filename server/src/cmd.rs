//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to `CHAT_COMMANDS`
//! and provide a handler function.

use crate::Server;
use common::{
    comp,
    msg::ServerMsg,
    npc::{get_npc_name, NpcKind},
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
            "spawn",
            "{} {} {d}",
            "/spawn <alignment> <entity> [amount] : Spawn a test entity",
            handle_spawn
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

fn handle_kill(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(entity)
        .map(|s| s.hp.set_to(0, comp::HealthSource::Suicide));
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

fn handle_spawn(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_align, opt_id, opt_amount) = scan_fmt!(&args, action.arg_fmt, String, NpcKind, String);
    // This should be just an enum and be handled with scan_fmt!
    let opt_agent = alignment_to_agent(&opt_align.unwrap_or(String::new()), entity);

    // Make sure the amount is either not provided or a valid value
    let opt_amount: Option<u32> = if let Some(amount) = opt_amount {
        match amount.parse().ok() {
            Some(x) if x == 0 => None,
            x => x
        }
    } else {
        Some(1)
    };

    match (opt_agent, opt_id, opt_amount) {
        (Some(agent), Some(id), Some(amount)) => {
            match server
                .state
                .read_component_cloned::<comp::phys::Pos>(entity)
            {
                Some(mut pos) => {
                    pos.0.x += 1.0; // Temp fix TODO: Solve NaN issue with positions of pets
                    let body = kind_to_body(id);
                    for _ in 0..amount {
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
        NpcKind::Humanoid => comp::Body::Humanoid(comp::HumanoidBody::random()),
        NpcKind::Pig => comp::Body::Quadruped(comp::QuadrupedBody::random()),
        NpcKind::Wolf => comp::Body::QuadrupedMedium(comp::QuadrupedMediumBody::random()),
    }
}
