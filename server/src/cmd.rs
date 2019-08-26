//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to `CHAT_COMMANDS`
//! and provide a handler function.

use crate::Server;
use chrono::{NaiveTime, Timelike};
use common::{
    comp,
    event::{Event as GameEvent, EventBus},
    msg::ServerMsg,
    npc::{get_npc_name, NpcKind},
    state::TimeOfDay,
    terrain::TerrainChunkSize,
    vol::VolSize,
};
use rand::Rng;
use specs::{Builder, Entity as EcsEntity, Join};
use vek::*;

use lazy_static::lazy_static;
use scan_fmt::{scan_fmt, scan_fmt_some};

/// Struct representing a command that a user can run from server chat.
pub struct ChatCommand {
    /// The keyword used to invoke the command, omitting the leading '/'.
    pub keyword: &'static str,
    /// A format string for parsing arguments.
    arg_fmt: &'static str,
    /// A message that explains how the command is used.
    help_string: &'static str,
    /// A boolean that is used to check whether the command requires administrator permissions or not.
    needs_admin: bool,
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
        needs_admin: bool,
        handler: fn(&mut Server, EcsEntity, String, &ChatCommand),
    ) -> Self {
        Self {
            keyword,
            arg_fmt,
            help_string,
            needs_admin,
            handler,
        }
    }
    /// Calls the contained handler function, passing `&self` as the last argument.
    pub fn execute(&self, server: &mut Server, entity: EcsEntity, args: String) {
        if self.needs_admin {
            if !server.entity_is_admin(entity) {
                server.clients.notify(
                    entity,
                    ServerMsg::private(String::from(
                        "Unavailable command '/{}'.\nType '/help' for available commands",
                    )),
                );
                return;
            } else {
                (self.handler)(server, entity, args, self);
            }
        } else {
            (self.handler)(server, entity, args, self);
        }
    }
}

lazy_static! {
    /// Static list of chat commands available to the server.
    pub static ref CHAT_COMMANDS: Vec<ChatCommand> = vec![
        ChatCommand::new(
            "jump",
            "{d} {d} {d}",
            "/jump <dx> <dy> <dz> : Offset your current position",
            true,
            handle_jump,
        ),
        ChatCommand::new(
            "goto",
            "{d} {d} {d}",
            "/goto <x> <y> <z> : Teleport to a position",
            true,
            handle_goto,
        ),
        ChatCommand::new(
            "alias",
            "{}",
            "/alias <name> : Change your alias",
            false,
            handle_alias,
        ),
        ChatCommand::new(
            "tp",
            "{}",
            "/tp <alias> : Teleport to another player",
            true,
            handle_tp,
        ),
        ChatCommand::new(
            "kill",
            "{}",
            "/kill : Kill yourself",
            false,
            handle_kill,
        ),
        ChatCommand::new(
            "time",
            "{} {s}",
            "/time <XY:XY> or [Time of day] : Set the time of day",
            true,
            handle_time,
        ),
        ChatCommand::new(
            "spawn",
            "{} {} {d}",
            "/spawn <alignment> <entity> [amount] : Spawn a test entity",
            true,
            handle_spawn,
        ),
        ChatCommand::new(
             "players",
             "{}",
             "/players : Lists players currently online",
             false,
             handle_players,
         ),
        ChatCommand::new(
            "help", "", "/help: Display this message", false, handle_help),
        ChatCommand::new(
            "health",
            "{}",
            "/health : Set your current health",
            true,
            handle_health,
        ),
        ChatCommand::new(
            "build",
            "",
            "/build : Toggles build mode on and off",
            true,
            handle_build,
        ),
        ChatCommand::new(
            "tell",
            "{}",
            "/tell <alias> <message>: Send a message to another player",
            false,
            handle_tell,
        ),
        ChatCommand::new(
            "killnpcs",
            "{}",
            "/killnpcs : Kill the NPCs",
            true,
            handle_killnpcs,
        ),
        ChatCommand::new(
            "object",
            "{}",
            "/object [Name]: Spawn an object",
            true,
            handle_object,
        ),
        ChatCommand::new(
            "light",
            "{} {} {} {} {} {} {}",
            "/light <opt:  <<cr> <cg> <cb>> <<ox> <oy> <oz>> <<strength>>>: Spawn entity with light",
            true,
            handle_light,
        ),
        ChatCommand::new(
            "lantern",
            "{}",
            "/lantern : adds/remove light near player",
            false,
            handle_lantern,
        ),
        ChatCommand::new(
            "explosion",
            "{}",
            "/explosion <radius> : Explodes the ground around you",
            false,
            handle_explosion,
        ),
        ChatCommand::new(
            "adminify",
            "{}",
            "/adminify <playername> : Temporarily gives a player admin permissions or removes them",
            true,
            handle_adminify,
        ),
        ChatCommand::new(
             "debug_column",
             "{} {}",
             "/debug_column <x> <y> : Prints some debug information about a column",
             false,
             handle_debug_column,
         ),
    ];
}

fn handle_jump(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    if let Ok((x, y, z)) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32) {
        match server.state.read_component_cloned::<comp::Pos>(entity) {
            Some(current_pos) => {
                server
                    .state
                    .write_component(entity, comp::Pos(current_pos.0 + Vec3::new(x, y, z)));
                server.state.write_component(entity, comp::ForceUpdate);
            }
            None => server.clients.notify(
                entity,
                ServerMsg::private(String::from("You have no position.")),
            ),
        }
    }
}

fn handle_goto(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    if let Ok((x, y, z)) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32) {
        if server
            .state
            .read_component_cloned::<comp::Pos>(entity)
            .is_some()
        {
            server
                .state
                .write_component(entity, comp::Pos(Vec3::new(x, y, z)));
            server.state.write_component(entity, comp::ForceUpdate);
        } else {
            server.clients.notify(
                entity,
                ServerMsg::private(String::from("You have no position.")),
            );
        }
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string)));
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
    let time = scan_fmt_some!(&args, action.arg_fmt, String);
    let new_time = match time.as_ref().map(|s| s.as_str()) {
        Some("night") => NaiveTime::from_hms(0, 0, 0),
        Some("dawn") => NaiveTime::from_hms(5, 0, 0),
        Some("day") => NaiveTime::from_hms(12, 0, 0),
        Some("dusk") => NaiveTime::from_hms(17, 0, 0),
        Some(n) => match n.parse() {
            Ok(n) => n,
            Err(_) => match NaiveTime::parse_from_str(n, "%H:%M") {
                Ok(time) => time,
                Err(_) => {
                    server.clients.notify(
                        entity,
                        ServerMsg::private(format!("'{}' is not a valid time.", n)),
                    );
                    return;
                }
            },
        },
        None => {
            let time_in_seconds = server.state.ecs_mut().read_resource::<TimeOfDay>().0;

            let current_time = NaiveTime::from_num_seconds_from_midnight_opt(
                // Wraps around back to 0s if it exceeds 24 hours (24 hours = 86400s)
                (time_in_seconds as u64 % 86400) as u32,
                0,
            );
            let msg = match current_time {
                Some(time) => format!("It is {}", time.format("%H:%M").to_string()),
                None => String::from("Unknown Time"),
            };
            server.clients.notify(entity, ServerMsg::private(msg));
            return;
        }
    };

    server.state.ecs_mut().write_resource::<TimeOfDay>().0 =
        new_time.num_seconds_from_midnight() as f64;

    server.clients.notify(
        entity,
        ServerMsg::private(format!(
            "Time changed to: {}",
            new_time.format("%H:%M").to_string()
        )),
    );
}

fn handle_health(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    if let Ok(hp) = scan_fmt!(&args, action.arg_fmt, u32) {
        if let Some(stats) = server
            .state
            .ecs_mut()
            .write_storage::<comp::Stats>()
            .get_mut(entity)
        {
            stats.health.set_to(hp, comp::HealthSource::Command);
        } else {
            server.clients.notify(
                entity,
                ServerMsg::private(String::from("You have no health.")),
            );
        }
    } else {
        server.clients.notify(
            entity,
            ServerMsg::private(String::from("You must specify health amount!")),
        );
    }
}

fn handle_alias(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    if let Ok(alias) = scan_fmt!(&args, action.arg_fmt, String) {
        server
            .state
            .ecs_mut()
            .write_storage::<comp::Player>()
            .get_mut(entity)
            .map(|player| player.alias = alias);
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string)));
    }
}

fn handle_tp(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    if let Ok(alias) = scan_fmt!(&args, action.arg_fmt, String) {
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
                        ServerMsg::private(format!("Unable to teleport to player '{}'!", alias)),
                    ),
                },
                None => {
                    server.clients.notify(
                        entity,
                        ServerMsg::private(format!("Player '{}' not found!", alias)),
                    );
                    server
                        .clients
                        .notify(entity, ServerMsg::private(String::from(action.help_string)));
                }
            },
            None => {
                server
                    .clients
                    .notify(entity, ServerMsg::private(format!("You have no position!")));
            }
        }
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string)));
    }
}

fn handle_spawn(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    match scan_fmt_some!(&args, action.arg_fmt, String, NpcKind, String) {
        (Some(opt_align), Some(id), opt_amount) => {
            if let Some(agent) = alignment_to_agent(&opt_align, entity) {
                let amount = opt_amount
                    .and_then(|a| a.parse().ok())
                    .filter(|x| *x > 0)
                    .unwrap_or(1)
                    .min(10);

                match server.state.read_component_cloned::<comp::Pos>(entity) {
                    Some(pos) => {
                        for _ in 0..amount {
                            let vel = Vec3::new(
                                rand::thread_rng().gen_range(-2.0, 3.0),
                                rand::thread_rng().gen_range(-2.0, 3.0),
                                10.0,
                            );

                            let body = kind_to_body(id);
                            server
                                .create_npc(pos, get_npc_name(id), body)
                                .with(comp::Vel(vel))
                                .with(agent)
                                .build();
                        }
                        server.clients.notify(
                            entity,
                            ServerMsg::private(format!("Spawned {} entities", amount).to_owned()),
                        );
                    }
                    None => server.clients.notify(
                        entity,
                        ServerMsg::private("You have no position!".to_owned()),
                    ),
                }
            }
        }
        _ => {
            server
                .clients
                .notify(entity, ServerMsg::private(String::from(action.help_string)));
        }
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
            .notify(entity, ServerMsg::private(header_message + &player_list));
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(header_message));
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
            ServerMsg::private(String::from("Toggled off build mode!")),
        );
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .insert(entity, comp::CanBuild);
        server.clients.notify(
            entity,
            ServerMsg::private(String::from("Toggled on build mode!")),
        );
    }
}

// TODO: Don't display commands that the player cannot use.
fn handle_help(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    for cmd in CHAT_COMMANDS.iter() {
        server
            .clients
            .notify(entity, ServerMsg::private(String::from(cmd.help_string)));
    }
}

fn alignment_to_agent(alignment: &str, target: EcsEntity) -> Option<comp::Agent> {
    match alignment {
        "hostile" => Some(comp::Agent::enemy()),
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
    server.clients.notify(entity, ServerMsg::private(text));
}

fn handle_object(server: &mut Server, entity: EcsEntity, args: String, _action: &ChatCommand) {
    let obj_type = scan_fmt!(&args, _action.arg_fmt, String);

    let pos = server
        .state
        .ecs()
        .read_storage::<comp::Pos>()
        .get(entity)
        .copied();
    let ori = server
        .state
        .ecs()
        .read_storage::<comp::Ori>()
        .get(entity)
        .copied();
    /*let builder = server
    .create_object(pos, ori, obj_type)
    .with(ori);*/
    if let (Some(pos), Some(ori)) = (pos, ori) {
        let obj_type = match obj_type.as_ref().map(String::as_str) {
            Ok("scarecrow") => comp::object::Body::Scarecrow,
            Ok("cauldron") => comp::object::Body::Cauldron,
            Ok("chest_vines") => comp::object::Body::ChestVines,
            Ok("chest") => comp::object::Body::Chest,
            Ok("chest_dark") => comp::object::Body::ChestDark,
            Ok("chest_demon") => comp::object::Body::ChestDemon,
            Ok("chest_gold") => comp::object::Body::ChestGold,
            Ok("chest_light") => comp::object::Body::ChestLight,
            Ok("chest_open") => comp::object::Body::ChestOpen,
            Ok("chest_skull") => comp::object::Body::ChestSkull,
            Ok("pumpkin") => comp::object::Body::Pumpkin,
            Ok("pumpkin_2") => comp::object::Body::Pumpkin2,
            Ok("pumpkin_3") => comp::object::Body::Pumpkin3,
            Ok("pumpkin_4") => comp::object::Body::Pumpkin4,
            Ok("pumpkin_5") => comp::object::Body::Pumpkin5,
            Ok("campfire") => comp::object::Body::Campfire,
            Ok("lantern_ground") => comp::object::Body::LanternGround,
            Ok("lantern_ground_open") => comp::object::Body::LanternGroundOpen,
            Ok("lantern_2") => comp::object::Body::LanternStanding2,
            Ok("lantern") => comp::object::Body::LanternStanding,
            Ok("potion_blue") => comp::object::Body::PotionBlue,
            Ok("potion_green") => comp::object::Body::PotionGreen,
            Ok("potion_red") => comp::object::Body::PotionRed,
            Ok("crate") => comp::object::Body::Crate,
            Ok("tent") => comp::object::Body::Tent,
            Ok("bomb") => comp::object::Body::Bomb,
            Ok("window_spooky") => comp::object::Body::WindowSpooky,
            Ok("door_spooky") => comp::object::Body::DoorSpooky,
            Ok("carpet") => comp::object::Body::Carpet,
            Ok("table_human") => comp::object::Body::Table,
            Ok("table_human_2") => comp::object::Body::Table2,
            Ok("table_human_3") => comp::object::Body::Table3,
            Ok("drawer") => comp::object::Body::Drawer,
            Ok("bed_human_blue") => comp::object::Body::BedBlue,
            Ok("anvil") => comp::object::Body::Anvil,
            Ok("gravestone") => comp::object::Body::Gravestone,
            Ok("gravestone_2") => comp::object::Body::Gravestone2,
            Ok("chair") => comp::object::Body::Chair,
            Ok("chair_2") => comp::object::Body::Chair2,
            Ok("chair_3") => comp::object::Body::Chair3,
            Ok("bench_human") => comp::object::Body::Bench,
            Ok("bedroll") => comp::object::Body::Bedroll,
            Ok("carpet_human_round") => comp::object::Body::CarpetHumanRound,
            Ok("carpet_human_square") => comp::object::Body::CarpetHumanSquare,
            Ok("carpet_human_square_2") => comp::object::Body::CarpetHumanSquare2,
            Ok("carpet_human_squircle") => comp::object::Body::CarpetHumanSquircle,
            _ => {
                return server.clients.notify(
                    entity,
                    ServerMsg::private(String::from("Object not found!")),
                );
            }
        };
        server
            .create_object(pos, obj_type)
            .with(comp::Ori(
                // converts player orientation into a 90° rotation for the object by using the axis with the highest value
                ori.0
                    .map(|e| {
                        if e.abs() == ori.0.map(|e| e.abs()).reduce_partial_max() {
                            e
                        } else {
                            0.0
                        }
                    })
                    .normalized(),
            ))
            .build();
        server
            .clients
            .notify(entity, ServerMsg::private(format!("Spawned object.")));
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(format!("You have no position!")));
    }
}

fn handle_light(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_r, opt_g, opt_b, opt_x, opt_y, opt_z, opt_s) =
        scan_fmt_some!(&args, action.arg_fmt, f32, f32, f32, f32, f32, f32, f32);

    let mut light_emitter = comp::LightEmitter::default();

    if let (Some(r), Some(g), Some(b)) = (opt_r, opt_g, opt_b) {
        let r = r.max(0.0).min(1.0);
        let g = g.max(0.0).min(1.0);
        let b = b.max(0.0).min(1.0);
        light_emitter.col = Rgb::new(r, g, b)
    };
    if let (Some(x), Some(y), Some(z)) = (opt_x, opt_y, opt_z) {
        light_emitter.offset = Vec3::new(x, y, z)
    };
    if let Some(s) = opt_s {
        light_emitter.strength = s.max(0.0)
    };
    let pos = server
        .state
        .ecs()
        .read_storage::<comp::Pos>()
        .get(entity)
        .copied();
    if let Some(pos) = pos {
        server
            .state
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::ForceUpdate)
            .with(light_emitter)
            .build();
        server
            .clients
            .notify(entity, ServerMsg::private(format!("Spawned object.")));
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(format!("You have no position!")));
    }
}

fn handle_lantern(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_s = scan_fmt_some!(&args, action.arg_fmt, f32);

    if server
        .state
        .read_storage::<comp::LightEmitter>()
        .get(entity)
        .is_some()
    {
        if let Some(s) = opt_s {
            if let Some(light) = server
                .state
                .ecs()
                .write_storage::<comp::LightEmitter>()
                .get_mut(entity)
            {
                light.strength = s.max(0.1).min(20.0);
                server.clients.notify(
                    entity,
                    ServerMsg::private(String::from("You played with flame strength.")),
                );
            }
        } else {
            server
                .state
                .ecs()
                .write_storage::<comp::LightEmitter>()
                .remove(entity);
            server.clients.notify(
                entity,
                ServerMsg::private(String::from("You put out the lantern.")),
            );
        }
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .insert(
                entity,
                comp::LightEmitter {
                    offset: Vec3::new(0.5, 0.2, 0.8),
                    col: Rgb::new(1.0, 0.75, 0.3),
                    strength: if let Some(s) = opt_s {
                        s.max(0.0).min(20.0)
                    } else {
                        6.0
                    },
                },
            );

        server.clients.notify(
            entity,
            ServerMsg::private(String::from("You lighted your lantern.")),
        );
    }
}

fn handle_explosion(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let radius = scan_fmt!(&args, action.arg_fmt, f32).unwrap_or(8.0);

    match server.state.read_component_cloned::<comp::Pos>(entity) {
        Some(pos) => server
            .state
            .ecs()
            .read_resource::<EventBus>()
            .emit(GameEvent::Explosion { pos: pos.0, radius }),
        None => server.clients.notify(
            entity,
            ServerMsg::private(String::from("You have no position!")),
        ),
    }
}

fn handle_adminify(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    if let Ok(alias) = scan_fmt!(&args, action.arg_fmt, String) {
        let ecs = server.state.ecs();
        let opt_player = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == alias)
            .map(|(entity, _)| entity);
        match opt_player {
            Some(player) => match server.state.read_component_cloned::<comp::Admin>(player) {
                Some(_admin) => {
                    ecs.write_storage::<comp::Admin>().remove(player);
                }
                None => {
                    server.state.write_component(player, comp::Admin);
                }
            },
            None => {
                server.clients.notify(
                    entity,
                    ServerMsg::private(format!("Player '{}' not found!", alias)),
                );
                server
                    .clients
                    .notify(entity, ServerMsg::private(String::from(action.help_string)));
            }
        }
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string)));
    }
}

fn handle_tell(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    if let Ok(alias) = scan_fmt!(&args, action.arg_fmt, String) {
        let ecs = server.state.ecs();
        let msg = &args[alias.len()..args.len()];
        if let Some(player) = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == alias)
            .map(|(entity, _)| entity)
        {
            if player != entity {
                if msg.len() > 1 {
                    if let Some(name) = ecs
                        .read_storage::<comp::Player>()
                        .get(entity)
                        .map(|s| s.alias.clone())
                    {
                        server.clients.notify(
                            player,
                            ServerMsg::tell(format!("[{}] tells you:{}", name, msg)),
                        );
                        server.clients.notify(
                            entity,
                            ServerMsg::tell(format!("You tell [{}]:{}", alias, msg)),
                        );
                    } else {
                        server.clients.notify(
                            entity,
                            ServerMsg::private(String::from("Failed to send message.")),
                        );
                    }
                } else {
                    server.clients.notify(
                        entity,
                        ServerMsg::private(format!("[{}] wants to talk to you.", alias)),
                    );
                }
            } else {
                server.clients.notify(
                    entity,
                    ServerMsg::private(format!("You can't /tell yourself.")),
                );
            }
        } else {
            server.clients.notify(
                entity,
                ServerMsg::private(format!("Player '{}' not found!", alias)),
            );
        }
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string)));
    }
}

fn handle_debug_column(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let sim = server.world.sim();
    if let Ok((x, y)) = scan_fmt!(&args, action.arg_fmt, i32, i32) {
        let wpos = Vec2::new(x, y);
        /* let chunk_pos = wpos.map2(Vec2::from(TerrainChunkSize::SIZE), |e, sz: u32| {
            e / sz as i32
        }); */

        let foo = || {
            // let sim_chunk = sim.get(chunk_pos)?;
            let alt_base = sim.get_interpolated(wpos, |chunk| chunk.alt_base)?;
            let alt = sim.get_interpolated(wpos, |chunk| chunk.alt)?;
            let chaos = sim.get_interpolated(wpos, |chunk| chunk.chaos)?;
            let temp = sim.get_interpolated(wpos, |chunk| chunk.temp)?;
            let humidity = sim.get_interpolated(wpos, |chunk| chunk.humidity)?;
            let rockiness = sim.get_interpolated(wpos, |chunk| chunk.rockiness)?;
            let tree_density = sim.get_interpolated(wpos, |chunk| chunk.tree_density)?;
            let spawn_rate = sim.get_interpolated(wpos, |chunk| chunk.spawn_rate)?;

            Some(format!(
                r#"wpos: {:?}
alt_base {:?}
alt {:?}
chaos {:?}
temp {:?}
humidity {:?}
rockiness {:?}
tree_density {:?}
spawn_rate {:?} "#,
                wpos, alt_base, alt, chaos, temp, humidity, rockiness, tree_density, spawn_rate
            ))
        };
        if let Some(s) = foo() {
            server.clients.notify(entity, ServerMsg::private(s));
        } else {
            server.clients.notify(
                entity,
                ServerMsg::private(String::from("Not a pregenerated chunk.")),
            );
        }
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string)));
    }
}
