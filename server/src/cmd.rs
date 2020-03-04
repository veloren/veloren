//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to
//! `CHAT_COMMANDS` and provide a handler function.

use crate::{Server, StateExt};
use chrono::{NaiveTime, Timelike};
use common::{
    assets, comp,
    event::{EventBus, ServerEvent},
    msg::{PlayerListUpdate, ServerMsg},
    npc::{self, get_npc_name},
    state::TimeOfDay,
    sync::{Uid, WorldSyncExt},
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use rand::Rng;
use specs::{Builder, Entity as EcsEntity, Join, WorldExt};
use vek::*;
use world::util::Sampler;

use lazy_static::lazy_static;
use log::error;
use scan_fmt::{scan_fmt, scan_fmt_some};

/// Struct representing a command that a user can run from server chat.
pub struct ChatCommand {
    /// The keyword used to invoke the command, omitting the leading '/'.
    pub keyword: &'static str,
    /// A format string for parsing arguments.
    arg_fmt: &'static str,
    /// A message that explains how the command is used.
    help_string: &'static str,
    /// A boolean that is used to check whether the command requires
    /// administrator permissions or not.
    needs_admin: bool,
    /// Handler function called when the command is executed.
    /// # Arguments
    /// * `&mut Server` - the `Server` instance executing the command.
    /// * `EcsEntity` - an `Entity` corresponding to the player that invoked the
    ///   command.
    /// * `String` - a `String` containing the part of the command after the
    ///   keyword.
    /// * `&ChatCommand` - the command to execute with the above arguments.
    /// Handler functions must parse arguments from the the given `String`
    /// (`scan_fmt!` is included for this purpose).
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

    /// Calls the contained handler function, passing `&self` as the last
    /// argument.
    pub fn execute(&self, server: &mut Server, entity: EcsEntity, args: String) {
        if self.needs_admin {
            if !server.entity_is_admin(entity) {
                server.notify_client(
                    entity,
                    ServerMsg::private(format!(
                        "You don't have permission to use '/{}'.",
                        self.keyword
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
            "give_item",
            "{d}",
            "/give_item <path to item>\n\
            Example: common/items/debug/boost",
            true,
            handle_give,),
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
            true,
            handle_explosion,
        ),
        ChatCommand::new(
            "waypoint",
            "{}",
            "/waypoint : Set your waypoint to your current position",
            false,
            handle_waypoint,
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
         ChatCommand::new(
             "give_exp",
             "{d} {}",
             "/give_exp <amount> <playername?> : Give experience to yourself or specify a target player",
             true,
             handle_exp,
         ),
         ChatCommand::new(
             "set_level",
             "{d} {}",
             "/set_level <level> <playername?> : Set own Level or specify a target player",
             true,
             handle_level
         ),
        ChatCommand::new(
             "removelights",
             "{}",
             "/removelights [radius] : Removes all lights spawned by players",
             true,
             handle_remove_lights,
         ),
        ChatCommand::new(
            "debug",
            "",
            "/debug : Place all debug items into your pack.",
            true,
            handle_debug,
        ),
    ];
}

fn handle_give(server: &mut Server, entity: EcsEntity, args: String, _action: &ChatCommand) {
    if let Ok(item) = assets::load_cloned(&args) {
        server
            .state
            .ecs()
            .write_storage::<comp::Inventory>()
            .get_mut(entity)
            .map(|inv| inv.push(item));
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::InventoryUpdate>()
            .insert(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Given),
            );
    } else {
        server.notify_client(entity, ServerMsg::private(String::from("Invalid item!")));
    }
}

fn handle_jump(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    if let Ok((x, y, z)) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32) {
        match server.state.read_component_cloned::<comp::Pos>(entity) {
            Some(current_pos) => {
                server
                    .state
                    .write_component(entity, comp::Pos(current_pos.0 + Vec3::new(x, y, z)));
                server.state.write_component(entity, comp::ForceUpdate);
            },
            None => server.notify_client(
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
            server.notify_client(
                entity,
                ServerMsg::private(String::from("You have no position.")),
            );
        }
    } else {
        server.notify_client(entity, ServerMsg::private(String::from(action.help_string)));
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
        Some("midnight") => NaiveTime::from_hms(0, 0, 0),
        Some("night") => NaiveTime::from_hms(20, 0, 0),
        Some("dawn") => NaiveTime::from_hms(5, 0, 0),
        Some("morning") => NaiveTime::from_hms(8, 0, 0),
        Some("day") => NaiveTime::from_hms(10, 0, 0),
        Some("noon") => NaiveTime::from_hms(12, 0, 0),
        Some("dusk") => NaiveTime::from_hms(17, 0, 0),
        Some(n) => match n.parse() {
            Ok(n) => n,
            Err(_) => match NaiveTime::parse_from_str(n, "%H:%M") {
                Ok(time) => time,
                Err(_) => {
                    server.notify_client(
                        entity,
                        ServerMsg::private(format!("'{}' is not a valid time.", n)),
                    );
                    return;
                },
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
            server.notify_client(entity, ServerMsg::private(msg));
            return;
        },
    };

    server.state.ecs_mut().write_resource::<TimeOfDay>().0 =
        new_time.num_seconds_from_midnight() as f64;

    server.notify_client(
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
            .ecs()
            .write_storage::<comp::Stats>()
            .get_mut(entity)
        {
            stats.health.set_to(hp, comp::HealthSource::Command);
        } else {
            server.notify_client(
                entity,
                ServerMsg::private(String::from("You have no health.")),
            );
        }
    } else {
        server.notify_client(
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

        // Update name on client player lists
        let ecs = server.state.ecs();
        if let (Some(uid), Some(player)) = (
            ecs.read_storage::<Uid>().get(entity),
            ecs.read_storage::<comp::Player>().get(entity),
        ) {
            let msg = ServerMsg::PlayerListUpdate(PlayerListUpdate::Alias(
                (*uid).into(),
                player.alias.clone(),
            ));
            server.state.notify_registered_clients(msg);
        }
    } else {
        server.notify_client(entity, ServerMsg::private(String::from(action.help_string)));
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
                    },
                    None => server.notify_client(
                        entity,
                        ServerMsg::private(format!("Unable to teleport to player '{}'!", alias)),
                    ),
                },
                None => {
                    server.notify_client(
                        entity,
                        ServerMsg::private(format!("Player '{}' not found!", alias)),
                    );
                    server.notify_client(
                        entity,
                        ServerMsg::private(String::from(action.help_string)),
                    );
                },
            },
            None => {
                server.notify_client(entity, ServerMsg::private(format!("You have no position!")));
            },
        }
    } else {
        server.notify_client(entity, ServerMsg::private(String::from(action.help_string)));
    }
}

fn handle_spawn(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    match scan_fmt_some!(&args, action.arg_fmt, String, npc::NpcBody, String) {
        (Some(opt_align), Some(npc::NpcBody(id, mut body)), opt_amount) => {
            if let Some(alignment) = parse_alignment(entity, &opt_align) {
                let amount = opt_amount
                    .and_then(|a| a.parse().ok())
                    .filter(|x| *x > 0)
                    .unwrap_or(1)
                    .min(10);

                match server.state.read_component_cloned::<comp::Pos>(entity) {
                    Some(pos) => {
                        let agent =
                            if let comp::Alignment::Owned(_) | comp::Alignment::Npc = alignment {
                                comp::Agent::default()
                            } else {
                                comp::Agent::default().with_patrol_origin(pos.0)
                            };

                        for _ in 0..amount {
                            let vel = Vec3::new(
                                rand::thread_rng().gen_range(-2.0, 3.0),
                                rand::thread_rng().gen_range(-2.0, 3.0),
                                10.0,
                            );

                            let body = body();

                            let new_entity = server
                                .state
                                .create_npc(
                                    pos,
                                    comp::Stats::new(get_npc_name(id).into(), body, None),
                                    body,
                                )
                                .with(comp::Vel(vel))
                                .with(comp::MountState::Unmounted)
                                .with(agent.clone())
                                .with(alignment)
                                .build();

                            if let Some(uid) = server.state.ecs().uid_from_entity(new_entity) {
                                server.notify_client(
                                    entity,
                                    ServerMsg::private(
                                        format!("Spawned entity with ID: {}", uid).to_owned(),
                                    ),
                                );
                            }
                        }
                        server.notify_client(
                            entity,
                            ServerMsg::private(format!("Spawned {} entities", amount).to_owned()),
                        );
                    },
                    None => server.notify_client(
                        entity,
                        ServerMsg::private("You have no position!".to_owned()),
                    ),
                }
            }
        },
        _ => {
            server.notify_client(entity, ServerMsg::private(String::from(action.help_string)));
        },
    }
}

fn handle_players(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    let ecs = server.state.ecs();
    let players = ecs.read_storage::<comp::Player>();
    let count = players.join().count();
    let header_message: String = format!("{} online players: \n", count);
    if count > 0 {
        let mut player_iter = players.join();
        let first = player_iter
            .next()
            .expect("Player iterator returned none.")
            .alias
            .to_owned();
        let player_list = player_iter.fold(first, |mut s, p| {
            s += ",\n";
            s += &p.alias;
            s
        });

        server.notify_client(entity, ServerMsg::private(header_message + &player_list));
    } else {
        server.notify_client(entity, ServerMsg::private(header_message));
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
        server.notify_client(
            entity,
            ServerMsg::private(String::from("Toggled off build mode!")),
        );
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .insert(entity, comp::CanBuild);
        server.notify_client(
            entity,
            ServerMsg::private(String::from("Toggled on build mode!")),
        );
    }
}

fn handle_help(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    for cmd in CHAT_COMMANDS.iter() {
        if !cmd.needs_admin || server.entity_is_admin(entity) {
            server.notify_client(entity, ServerMsg::private(String::from(cmd.help_string)));
        }
    }
}

fn parse_alignment(owner: EcsEntity, alignment: &str) -> Option<comp::Alignment> {
    match alignment {
        "wild" => Some(comp::Alignment::Wild),
        "enemy" => Some(comp::Alignment::Enemy),
        "npc" => Some(comp::Alignment::Npc),
        "pet" => Some(comp::Alignment::Owned(owner)),
        _ => None,
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
    server.notify_client(entity, ServerMsg::private(text));
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
        let obj_str_res = obj_type.as_ref().map(String::as_str);
        let obj_type = match obj_str_res {
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
            Ok("campfire_lit") => comp::object::Body::CampfireLit,
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
            Ok("crafting_bench") => comp::object::Body::CraftingBench,
            _ => {
                return server.notify_client(
                    entity,
                    ServerMsg::private(String::from("Object not found!")),
                );
            },
        };
        server
            .create_object(pos, obj_type)
            .with(comp::Ori(
                // converts player orientation into a 90° rotation for the object by using the axis
                // with the highest value
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
        server.notify_client(
            entity,
            ServerMsg::private(format!(
                "Spawned: {}",
                obj_str_res.unwrap_or("<Unknown object>")
            )),
        );
    } else {
        server.notify_client(entity, ServerMsg::private(format!("You have no position!")));
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
        server.notify_client(entity, ServerMsg::private(format!("Spawned object.")));
    } else {
        server.notify_client(entity, ServerMsg::private(format!("You have no position!")));
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
                light.strength = s.max(0.1).min(10.0);
                server.notify_client(
                    entity,
                    ServerMsg::private(String::from("You adjusted flame strength.")),
                );
            }
        } else {
            server
                .state
                .ecs()
                .write_storage::<comp::LightEmitter>()
                .remove(entity);
            server.notify_client(
                entity,
                ServerMsg::private(String::from("You put out the lantern.")),
            );
        }
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .insert(entity, comp::LightEmitter {
                offset: Vec3::new(0.5, 0.2, 0.8),
                col: Rgb::new(1.0, 0.75, 0.3),
                strength: if let Some(s) = opt_s {
                    s.max(0.0).min(10.0)
                } else {
                    3.0
                },
            });

        server.notify_client(
            entity,
            ServerMsg::private(String::from("You lit your lantern.")),
        );
    }
}

fn handle_explosion(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let radius = scan_fmt!(&args, action.arg_fmt, f32).unwrap_or(8.0);

    match server.state.read_component_cloned::<comp::Pos>(entity) {
        Some(pos) => server
            .state
            .ecs()
            .read_resource::<EventBus<ServerEvent>>()
            .emit(ServerEvent::Explosion { pos: pos.0, radius }),
        None => server.notify_client(
            entity,
            ServerMsg::private(String::from("You have no position!")),
        ),
    }
}

fn handle_waypoint(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    match server.state.read_component_cloned::<comp::Pos>(entity) {
        Some(pos) => {
            let _ = server
                .state
                .ecs()
                .write_storage::<comp::Waypoint>()
                .insert(entity, comp::Waypoint::new(pos.0));
            server.notify_client(entity, ServerMsg::private(String::from("Waypoint set!")));
        },
        None => server.notify_client(
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
                },
                None => {
                    server.state.write_component(player, comp::Admin);
                },
            },
            None => {
                server.notify_client(
                    entity,
                    ServerMsg::private(format!("Player '{}' not found!", alias)),
                );
                server.notify_client(entity, ServerMsg::private(String::from(action.help_string)));
            },
        }
    } else {
        server.notify_client(entity, ServerMsg::private(String::from(action.help_string)));
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
                        server.notify_client(
                            player,
                            ServerMsg::tell(format!("[{}] tells:{}", name, msg)),
                        );
                        server.notify_client(
                            entity,
                            ServerMsg::tell(format!("To [{}]:{}", alias, msg)),
                        );
                    } else {
                        server.notify_client(
                            entity,
                            ServerMsg::private(String::from("Failed to send message.")),
                        );
                    }
                } else {
                    server.notify_client(
                        entity,
                        ServerMsg::private(format!("[{}] wants to talk to you.", alias)),
                    );
                }
            } else {
                server.notify_client(
                    entity,
                    ServerMsg::private(format!("You can't /tell yourself.")),
                );
            }
        } else {
            server.notify_client(
                entity,
                ServerMsg::private(format!("Player '{}' not found!", alias)),
            );
        }
    } else {
        server.notify_client(entity, ServerMsg::private(String::from(action.help_string)));
    }
}

#[cfg(not(feature = "worldgen"))]
fn handle_debug_column(
    server: &mut Server,
    entity: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    server.notify_client(
        entity,
        ServerMsg::private(String::from("Unsupported without worldgen enabled")),
    );
}

#[cfg(feature = "worldgen")]
fn handle_debug_column(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let sim = server.world.sim();
    let sampler = server.world.sample_columns();
    if let Ok((x, y)) = scan_fmt!(&args, action.arg_fmt, i32, i32) {
        let wpos = Vec2::new(x, y);
        /* let chunk_pos = wpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| {
            e / sz as i32
        }); */

        let foo = || {
            // let sim_chunk = sim.get(chunk_pos)?;
            let alt = sim.get_interpolated(wpos, |chunk| chunk.alt)?;
            let basement = sim.get_interpolated(wpos, |chunk| chunk.basement)?;
            let water_alt = sim.get_interpolated(wpos, |chunk| chunk.water_alt)?;
            let chaos = sim.get_interpolated(wpos, |chunk| chunk.chaos)?;
            let temp = sim.get_interpolated(wpos, |chunk| chunk.temp)?;
            let humidity = sim.get_interpolated(wpos, |chunk| chunk.humidity)?;
            let rockiness = sim.get_interpolated(wpos, |chunk| chunk.rockiness)?;
            let tree_density = sim.get_interpolated(wpos, |chunk| chunk.tree_density)?;
            let spawn_rate = sim.get_interpolated(wpos, |chunk| chunk.spawn_rate)?;
            let chunk_pos = wpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32);
            let chunk = sim.get(chunk_pos)?;
            let col = sampler.get(wpos)?;
            let downhill = chunk.downhill;
            let river = &chunk.river;
            let flux = chunk.flux;

            Some(format!(
                r#"wpos: {:?}
alt {:?} ({:?})
water_alt {:?} ({:?})
basement {:?}
river {:?}
downhill {:?}
chaos {:?}
flux {:?}
temp {:?}
humidity {:?}
rockiness {:?}
tree_density {:?}
spawn_rate {:?} "#,
                wpos,
                alt,
                col.alt,
                water_alt,
                col.water_level,
                basement,
                river,
                downhill,
                chaos,
                flux,
                temp,
                humidity,
                rockiness,
                tree_density,
                spawn_rate
            ))
        };
        if let Some(s) = foo() {
            server.notify_client(entity, ServerMsg::private(s));
        } else {
            server.notify_client(
                entity,
                ServerMsg::private(String::from("Not a pregenerated chunk.")),
            );
        }
    } else {
        server.notify_client(entity, ServerMsg::private(String::from(action.help_string)));
    }
}

fn find_target(
    ecs: &specs::World,
    opt_alias: Option<String>,
    fallback: EcsEntity,
) -> Result<EcsEntity, ServerMsg> {
    if let Some(alias) = opt_alias {
        (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == alias)
            .map(|(entity, _)| entity)
            .ok_or(ServerMsg::private(format!("Player '{}' not found!", alias)))
    } else {
        Ok(fallback)
    }
}

fn handle_exp(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (a_exp, a_alias) = scan_fmt_some!(&args, action.arg_fmt, i64, String);

    if let Some(exp) = a_exp {
        let ecs = server.state.ecs_mut();
        let target = find_target(&ecs, a_alias, entity);

        let mut error_msg = None;

        match target {
            Ok(player) => {
                if let Some(stats) = ecs.write_storage::<comp::Stats>().get_mut(player) {
                    stats.exp.change_by(exp);
                } else {
                    error_msg = Some(ServerMsg::private(String::from("Player has no stats!")));
                }
            },
            Err(e) => {
                error_msg = Some(e);
            },
        }

        if let Some(msg) = error_msg {
            server.notify_client(entity, msg);
        }
    }
}

fn handle_level(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (a_lvl, a_alias) = scan_fmt_some!(&args, action.arg_fmt, u32, String);

    if let Some(lvl) = a_lvl {
        let ecs = server.state.ecs_mut();
        let target = find_target(&ecs, a_alias, entity);

        let mut error_msg = None;

        match target {
            Ok(player) => {
                if let Some(stats) = ecs.write_storage::<comp::Stats>().get_mut(player) {
                    stats.level.set_level(lvl);

                    stats.update_max_hp();
                    stats
                        .health
                        .set_to(stats.health.maximum(), comp::HealthSource::LevelUp);
                } else {
                    error_msg = Some(ServerMsg::private(String::from("Player has no stats!")));
                }
            },
            Err(e) => {
                error_msg = Some(e);
            },
        }

        if let Some(msg) = error_msg {
            server.notify_client(entity, msg);
        }
    }
}

use common::comp::Item;
fn handle_debug(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    if let Ok(items) = assets::load_glob::<Item>("common.items.debug.*") {
        server
            .state()
            .ecs()
            .write_storage::<comp::Inventory>()
            .get_mut(entity)
            // TODO: Consider writing a `load_glob_cloned` in `assets` and using that here
            .map(|inv| inv.push_all_unique(items.iter().map(|item| item.as_ref().clone())));
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::InventoryUpdate>()
            .insert(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Debug),
            );
    } else {
        server.notify_client(
            entity,
            ServerMsg::private(String::from(
                "Debug items not found? Something is very broken.",
            )),
        );
    }
}

fn handle_remove_lights(
    server: &mut Server,
    entity: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let opt_radius = scan_fmt_some!(&args, action.arg_fmt, f32);
    let opt_player_pos = server.state.read_component_cloned::<comp::Pos>(entity);
    let mut to_delete = vec![];

    match opt_player_pos {
        Some(player_pos) => {
            let ecs = server.state.ecs();
            for (entity, pos, _, _, _) in (
                &ecs.entities(),
                &ecs.read_storage::<comp::Pos>(),
                &ecs.read_storage::<comp::LightEmitter>(),
                !&ecs.read_storage::<comp::WaypointArea>(),
                !&ecs.read_storage::<comp::Player>(),
            )
                .join()
            {
                if opt_radius
                    .map(|r| pos.0.distance(player_pos.0) < r)
                    .unwrap_or(true)
                {
                    to_delete.push(entity);
                }
            }
        },
        None => server.notify_client(
            entity,
            ServerMsg::private(String::from("You have no position.")),
        ),
    }

    let size = to_delete.len();

    for entity in to_delete {
        if let Err(err) = server.state.delete_entity_recorded(entity) {
            error!("Failed to delete light: {:?}", err);
        }
    }

    server.notify_client(
        entity,
        ServerMsg::private(String::from(format!("Removed {} lights!", size))),
    );
}
