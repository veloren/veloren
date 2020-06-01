//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to
//! `CHAT_COMMANDS` and provide a handler function.

use crate::{Server, StateExt};
use chrono::{NaiveTime, Timelike};
use common::{
    assets,
    cmd::{ChatCommand, CHAT_COMMANDS},
    comp,
    event::{EventBus, ServerEvent},
    msg::{Notification, PlayerListUpdate, ServerMsg},
    npc::{self, get_npc_name},
    state::TimeOfDay,
    sync::{Uid, WorldSyncExt},
    terrain::TerrainChunkSize,
    util::Dir,
    vol::RectVolSize,
};
use rand::Rng;
use specs::{Builder, Entity as EcsEntity, Join, WorldExt};
use vek::*;
use world::util::Sampler;

use log::error;
use scan_fmt::{scan_fmt, scan_fmt_some};

pub trait ChatCommandExt {
    fn execute(&self, server: &mut Server, entity: EcsEntity, args: String);
}
impl ChatCommandExt for ChatCommand {
    fn execute(&self, server: &mut Server, entity: EcsEntity, args: String) {
        let cmd_data = self.data();
        if cmd_data.needs_admin && !server.entity_is_admin(entity) {
            server.notify_client(
                entity,
                ServerMsg::private(format!(
                    "You don't have permission to use '/{}'.",
                    self.keyword()
                )),
            );
            return;
        } else {
            get_handler(self)(server, entity, entity, args, &self);
        }
    }
}

/// Handler function called when the command is executed.
/// # Arguments
/// * `&mut Server` - the `Server` instance executing the command.
/// * `EcsEntity` - an `Entity` corresponding to the player that invoked the
///   command.
/// * `EcsEntity` - an `Entity` for the player on whom the command is invoked.
///   This differs from the previous argument when using /sudo
/// * `String` - a `String` containing the part of the command after the
///   keyword.
/// * `&ChatCommand` - the command to execute with the above arguments.
/// Handler functions must parse arguments from the the given `String`
/// (`scan_fmt!` is included for this purpose).
type CommandHandler = fn(&mut Server, EcsEntity, EcsEntity, String, &ChatCommand);
fn get_handler(cmd: &ChatCommand) -> CommandHandler {
    match cmd {
        ChatCommand::Adminify => handle_adminify,
        ChatCommand::Alias => handle_alias,
        ChatCommand::Build => handle_build,
        ChatCommand::Debug => handle_debug,
        ChatCommand::DebugColumn => handle_debug_column,
        ChatCommand::Explosion => handle_explosion,
        ChatCommand::GiveExp => handle_give_exp,
        ChatCommand::GiveItem => handle_give_item,
        ChatCommand::Goto => handle_goto,
        ChatCommand::Health => handle_health,
        ChatCommand::Help => handle_help,
        ChatCommand::Jump => handle_jump,
        ChatCommand::Kill => handle_kill,
        ChatCommand::KillNpcs => handle_kill_npcs,
        ChatCommand::Lantern => handle_lantern,
        ChatCommand::Light => handle_light,
        ChatCommand::Object => handle_object,
        ChatCommand::Players => handle_players,
        ChatCommand::RemoveLights => handle_remove_lights,
        ChatCommand::SetLevel => handle_set_level,
        ChatCommand::Spawn => handle_spawn,
        ChatCommand::Sudo => handle_sudo,
        ChatCommand::Tell => handle_tell,
        ChatCommand::Time => handle_time,
        ChatCommand::Tp => handle_tp,
        ChatCommand::Version => handle_version,
        ChatCommand::Waypoint => handle_waypoint,
    }
}

fn handle_give_item(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let (Some(item_name), give_amount_opt) =
        scan_fmt_some!(&args, &action.arg_fmt(), String, u32)
    {
        let give_amount = give_amount_opt.unwrap_or(1);
        if let Ok(item) = assets::load_cloned(&item_name) {
            let mut item: Item = item;
            if let Ok(()) = item.set_amount(give_amount.min(2000)) {
                server
                    .state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(target)
                    .map(|inv| {
                        if inv.push(item).is_some() {
                            server.notify_client(
                                client,
                                ServerMsg::private(format!(
                                    "Player inventory full. Gave 0 of {} items.",
                                    give_amount
                                )),
                            );
                        }
                    });
            } else {
                // This item can't stack. Give each item in a loop.
                server
                    .state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(target)
                    .map(|inv| {
                        for i in 0..give_amount {
                            if inv.push(item.clone()).is_some() {
                                server.notify_client(
                                    client,
                                    ServerMsg::private(format!(
                                        "Player inventory full. Gave {} of {} items.",
                                        i, give_amount
                                    )),
                                );
                                break;
                            }
                        }
                    });
            }

            let _ = server
                .state
                .ecs()
                .write_storage::<comp::InventoryUpdate>()
                .insert(
                    target,
                    comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Given),
                );
        } else {
            server.notify_client(
                client,
                ServerMsg::private(format!("Invalid item: {}", item_name)),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(action.help_string())),
        );
    }
}

fn handle_jump(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Ok((x, y, z)) = scan_fmt!(&args, &action.arg_fmt(), f32, f32, f32) {
        match server.state.read_component_cloned::<comp::Pos>(target) {
            Some(current_pos) => {
                server
                    .state
                    .write_component(target, comp::Pos(current_pos.0 + Vec3::new(x, y, z)));
                server.state.write_component(target, comp::ForceUpdate);
            },
            None => server.notify_client(
                client,
                ServerMsg::private(String::from("You have no position.")),
            ),
        }
    }
}

fn handle_goto(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Ok((x, y, z)) = scan_fmt!(&args, &action.arg_fmt(), f32, f32, f32) {
        if server
            .state
            .read_component_cloned::<comp::Pos>(target)
            .is_some()
        {
            server
                .state
                .write_component(target, comp::Pos(Vec3::new(x, y, z)));
            server.state.write_component(target, comp::ForceUpdate);
        } else {
            server.notify_client(
                client,
                ServerMsg::private(String::from("You have no position.")),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(action.help_string())),
        );
    }
}

fn handle_kill(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    let reason = if client == target {
        comp::HealthSource::Suicide
    } else {
        if let Some(uid) = server.state.read_storage::<Uid>().get(client) {
            comp::HealthSource::Attack { by: *uid }
        } else {
            comp::HealthSource::Command
        }
    };
    server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(target)
        .map(|s| s.health.set_to(0, reason));
}

fn handle_time(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let time = scan_fmt_some!(&args, &action.arg_fmt(), String);
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
                        client,
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
            server.notify_client(client, ServerMsg::private(msg));
            return;
        },
    };

    server.state.ecs_mut().write_resource::<TimeOfDay>().0 =
        new_time.num_seconds_from_midnight() as f64;

    server.notify_client(
        client,
        ServerMsg::private(format!(
            "Time changed to: {}",
            new_time.format("%H:%M").to_string()
        )),
    );
}

fn handle_health(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Ok(hp) = scan_fmt!(&args, &action.arg_fmt(), u32) {
        if let Some(stats) = server
            .state
            .ecs()
            .write_storage::<comp::Stats>()
            .get_mut(target)
        {
            stats.health.set_to(hp, comp::HealthSource::Command);
        } else {
            server.notify_client(
                client,
                ServerMsg::private(String::from("You have no health.")),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from("You must specify health amount!")),
        );
    }
}

fn handle_alias(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Ok(alias) = scan_fmt!(&args, &action.arg_fmt(), String) {
        let old_alias_optional = server
            .state
            .ecs_mut()
            .write_storage::<comp::Player>()
            .get_mut(target)
            .map(|player| std::mem::replace(&mut player.alias, alias));

        // Update name on client player lists
        let ecs = server.state.ecs();
        if let (Some(uid), Some(player), Some(old_alias)) = (
            ecs.read_storage::<Uid>().get(target),
            ecs.read_storage::<comp::Player>().get(target),
            old_alias_optional,
        ) {
            let msg = ServerMsg::PlayerListUpdate(PlayerListUpdate::Alias(
                (*uid).into(),
                player.alias.clone(),
            ));
            server.state.notify_registered_clients(msg);

            // Announce alias change if target has a Body.
            if ecs.read_storage::<comp::Body>().get(target).is_some() {
                server
                    .state
                    .notify_registered_clients(ServerMsg::broadcast(format!(
                        "{} is now known as {}.",
                        old_alias, player.alias
                    )));
            }
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(action.help_string())),
        );
    }
}

fn handle_tp(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let opt_player = if let Some(alias) = scan_fmt_some!(&args, &action.arg_fmt(), String) {
        let ecs = server.state.ecs();
        (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == alias)
            .map(|(entity, _)| entity)
    } else {
        if client != target {
            Some(client)
        } else {
            server.notify_client(
                client,
                ServerMsg::private("You must specify a player name".to_string()),
            );
            server.notify_client(
                client,
                ServerMsg::private(String::from(action.help_string())),
            );
            return;
        }
    };
    if let Some(_pos) = server.state.read_component_cloned::<comp::Pos>(target) {
        if let Some(player) = opt_player {
            if let Some(pos) = server.state.read_component_cloned::<comp::Pos>(player) {
                server.state.write_component(target, pos);
                server.state.write_component(target, comp::ForceUpdate);
            } else {
                server.notify_client(
                    client,
                    ServerMsg::private(format!("Unable to teleport to player!")),
                );
            }
        } else {
            server.notify_client(client, ServerMsg::private(format!("Player not found!")));
            server.notify_client(
                client,
                ServerMsg::private(String::from(action.help_string())),
            );
        }
    } else {
        server.notify_client(client, ServerMsg::private(format!("You have no position!")));
    }
}

fn handle_spawn(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    match scan_fmt_some!(&args, &action.arg_fmt(), String, npc::NpcBody, String) {
        (Some(opt_align), Some(npc::NpcBody(id, mut body)), opt_amount) => {
            if let Some(alignment) = parse_alignment(target, &opt_align) {
                let amount = opt_amount
                    .and_then(|a| a.parse().ok())
                    .filter(|x| *x > 0)
                    .unwrap_or(1)
                    .min(10);

                match server.state.read_component_cloned::<comp::Pos>(target) {
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
                                    comp::Stats::new(get_npc_name(id).into(), body),
                                    comp::Loadout::default(),
                                    body,
                                )
                                .with(comp::Vel(vel))
                                .with(comp::MountState::Unmounted)
                                .with(agent.clone())
                                .with(alignment)
                                .build();

                            if let Some(uid) = server.state.ecs().uid_from_entity(new_entity) {
                                server.notify_client(
                                    client,
                                    ServerMsg::private(
                                        format!("Spawned entity with ID: {}", uid).to_owned(),
                                    ),
                                );
                            }
                        }
                        server.notify_client(
                            client,
                            ServerMsg::private(format!("Spawned {} entities", amount).to_owned()),
                        );
                    },
                    None => server.notify_client(
                        client,
                        ServerMsg::private("You have no position!".to_owned()),
                    ),
                }
            }
        },
        _ => {
            server.notify_client(
                client,
                ServerMsg::private(String::from(action.help_string())),
            );
        },
    }
}

fn handle_players(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    let ecs = server.state.ecs();

    let entity_tuples = (
        &ecs.entities(),
        &ecs.read_storage::<comp::Player>(),
        &ecs.read_storage::<comp::Stats>(),
    );

    server.notify_client(
        client,
        ServerMsg::private(entity_tuples.join().fold(
            format!("{} online players:", entity_tuples.join().count()),
            |s, (_, player, stat)| {
                format!(
                    "{}\n[{}]{} Lvl {}",
                    s,
                    player.alias,
                    stat.name,
                    stat.level.level()
                )
            },
        )),
    );
}

fn handle_build(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    if server
        .state
        .read_storage::<comp::CanBuild>()
        .get(target)
        .is_some()
    {
        server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .remove(target);
        server.notify_client(
            client,
            ServerMsg::private(String::from("Toggled off build mode!")),
        );
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .insert(target, comp::CanBuild);
        server.notify_client(
            client,
            ServerMsg::private(String::from("Toggled on build mode!")),
        );
    }
}

fn handle_help(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Some(cmd) = scan_fmt_some!(&args, &action.arg_fmt(), ChatCommand) {
        server.notify_client(client, ServerMsg::private(String::from(cmd.help_string())));
    } else {
        for cmd in CHAT_COMMANDS.iter() {
            if !cmd.needs_admin() || server.entity_is_admin(client) {
                server.notify_client(client, ServerMsg::private(String::from(cmd.help_string())));
            }
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

fn handle_kill_npcs(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
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
    server.notify_client(client, ServerMsg::private(text));
}

fn handle_object(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let obj_type = scan_fmt!(&args, &action.arg_fmt(), String);

    let pos = server
        .state
        .ecs()
        .read_storage::<comp::Pos>()
        .get(target)
        .copied();
    let ori = server
        .state
        .ecs()
        .read_storage::<comp::Ori>()
        .get(target)
        .copied();
    /*let builder = server.state
    .create_object(pos, ori, obj_type)
    .with(ori);*/
    if let (Some(pos), Some(ori)) = (pos, ori) {
        let obj_str_res = obj_type.as_ref().map(String::as_str);
        if let Some(obj_type) = comp::object::ALL_OBJECTS
            .iter()
            .find(|o| Ok(o.to_string()) == obj_str_res)
        {
            server
                .state
                .create_object(pos, *obj_type)
                .with(comp::Ori(
                    // converts player orientation into a 90° rotation for the object by using the
                    // axis with the highest value
                    Dir::from_unnormalized(ori.0.map(|e| {
                        if e.abs() == ori.0.map(|e| e.abs()).reduce_partial_max() {
                            e
                        } else {
                            0.0
                        }
                    }))
                    .unwrap_or_default(),
                ))
                .build();
            server.notify_client(
                client,
                ServerMsg::private(format!(
                    "Spawned: {}",
                    obj_str_res.unwrap_or("<Unknown object>")
                )),
            );
        } else {
            return server.notify_client(
                client,
                ServerMsg::private(String::from("Object not found!")),
            );
        }
    } else {
        server.notify_client(client, ServerMsg::private(format!("You have no position!")));
    }
}

fn handle_light(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let (opt_r, opt_g, opt_b, opt_x, opt_y, opt_z, opt_s) =
        scan_fmt_some!(&args, &action.arg_fmt(), f32, f32, f32, f32, f32, f32, f32);

    let mut light_emitter = comp::LightEmitter::default();
    let mut light_offset_opt = None;

    if let (Some(r), Some(g), Some(b)) = (opt_r, opt_g, opt_b) {
        if r < 0.0 || g < 0.0 || b < 0.0 {
            server.notify_client(
                client,
                ServerMsg::private(String::from("cr, cg and cb values mustn't be negative.")),
            );
            return;
        }

        let r = r.max(0.0).min(1.0);
        let g = g.max(0.0).min(1.0);
        let b = b.max(0.0).min(1.0);
        light_emitter.col = Rgb::new(r, g, b)
    };
    if let (Some(x), Some(y), Some(z)) = (opt_x, opt_y, opt_z) {
        light_offset_opt = Some(comp::LightAnimation {
            offset: Vec3::new(x, y, z),
            col: light_emitter.col,
            strength: 0.0,
        })
    };
    if let Some(s) = opt_s {
        light_emitter.strength = s.max(0.0)
    };
    let pos = server
        .state
        .ecs()
        .read_storage::<comp::Pos>()
        .get(target)
        .copied();
    if let Some(pos) = pos {
        let builder = server
            .state
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::ForceUpdate)
            .with(light_emitter);
        if let Some(light_offset) = light_offset_opt {
            builder.with(light_offset).build();
        } else {
            builder.build();
        }
        server.notify_client(client, ServerMsg::private(format!("Spawned object.")));
    } else {
        server.notify_client(client, ServerMsg::private(format!("You have no position!")));
    }
}

fn handle_lantern(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let (Some(s), r, g, b) = scan_fmt_some!(&args, &action.arg_fmt(), f32, f32, f32, f32) {
        if let Some(light) = server
            .state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .get_mut(target)
        {
            light.strength = s.max(0.1).min(10.0);
            if let (Some(r), Some(g), Some(b)) = (r, g, b) {
                light.col = (
                    r.max(0.0).min(1.0),
                    g.max(0.0).min(1.0),
                    b.max(0.0).min(1.0),
                )
                    .into();
                server.notify_client(
                    client,
                    ServerMsg::private(String::from("You adjusted flame strength and color.")),
                );
            } else {
                server.notify_client(
                    client,
                    ServerMsg::private(String::from("You adjusted flame strength.")),
                );
            }
        } else {
            server.notify_client(
                client,
                ServerMsg::private(String::from("Please equip a lantern first")),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(action.help_string())),
        );
    }
}

fn handle_explosion(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let power = scan_fmt!(&args, &action.arg_fmt(), f32).unwrap_or(8.0);

    if power > 512.0 {
        server.notify_client(
            client,
            ServerMsg::private(String::from("Explosion power mustn't be more than 512.")),
        );
        return;
    } else if power <= 0.0 {
        server.notify_client(
            client,
            ServerMsg::private(String::from("Explosion power must be more than 0.")),
        );
        return;
    }

    let ecs = server.state.ecs();

    match server.state.read_component_cloned::<comp::Pos>(target) {
        Some(pos) => {
            ecs.read_resource::<EventBus<ServerEvent>>()
                .emit_now(ServerEvent::Explosion {
                    pos: pos.0,
                    power,
                    owner: ecs.read_storage::<Uid>().get(target).copied(),
                })
        },
        None => server.notify_client(
            client,
            ServerMsg::private(String::from("You have no position!")),
        ),
    }
}

fn handle_waypoint(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    match server.state.read_component_cloned::<comp::Pos>(target) {
        Some(pos) => {
            let time = server.state.ecs().read_resource();
            let _ = server
                .state
                .ecs()
                .write_storage::<comp::Waypoint>()
                .insert(target, comp::Waypoint::new(pos.0, *time));
            server.notify_client(client, ServerMsg::private(String::from("Waypoint saved!")));
            server.notify_client(client, ServerMsg::Notification(Notification::WaypointSaved));
        },
        None => server.notify_client(
            client,
            ServerMsg::private(String::from("You have no position!")),
        ),
    }
}

fn handle_adminify(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Ok(alias) = scan_fmt!(&args, &action.arg_fmt(), String) {
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
                    client,
                    ServerMsg::private(format!("Player '{}' not found!", alias)),
                );
                server.notify_client(
                    client,
                    ServerMsg::private(String::from(action.help_string())),
                );
            },
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(action.help_string())),
        );
    }
}

fn handle_tell(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if client != target {
        server.notify_client(
            client,
            ServerMsg::tell(String::from("It's rude to impersonate people")),
        );
        return;
    }
    if let Ok(alias) = scan_fmt!(&args, &action.arg_fmt(), String) {
        let ecs = server.state.ecs();
        let msg = &args[alias.len()..args.len()];
        if let Some(player) = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == alias)
            .map(|(entity, _)| entity)
        {
            if player != target {
                if msg.len() > 1 {
                    if let Some(name) = ecs
                        .read_storage::<comp::Player>()
                        .get(target)
                        .map(|s| s.alias.clone())
                    {
                        server.notify_client(
                            player,
                            ServerMsg::tell(format!("[{}] tells:{}", name, msg)),
                        );
                        server.notify_client(
                            client,
                            ServerMsg::tell(format!("To [{}]:{}", alias, msg)),
                        );
                    } else {
                        server.notify_client(
                            client,
                            ServerMsg::private(String::from("Failed to send message.")),
                        );
                    }
                } else {
                    server.notify_client(
                        client,
                        ServerMsg::private(format!("[{}] wants to talk to you.", alias)),
                    );
                }
            } else {
                server.notify_client(
                    client,
                    ServerMsg::private(format!("You can't /tell yourself.")),
                );
            }
        } else {
            server.notify_client(
                client,
                ServerMsg::private(format!("Player '{}' not found!", alias)),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(action.help_string())),
        );
    }
}

#[cfg(not(feature = "worldgen"))]
fn handle_debug_column(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    server.notify_client(
        client,
        ServerMsg::private(String::from("Unsupported without worldgen enabled")),
    );
}

#[cfg(feature = "worldgen")]
fn handle_debug_column(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let sim = server.world.sim();
    let sampler = server.world.sample_columns();
    if let Ok((x, y)) = scan_fmt!(&args, &action.arg_fmt(), i32, i32) {
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
            server.notify_client(client, ServerMsg::private(s));
        } else {
            server.notify_client(
                client,
                ServerMsg::private(String::from("Not a pregenerated chunk.")),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(action.help_string())),
        );
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

fn handle_give_exp(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let (a_exp, a_alias) = scan_fmt_some!(&args, &action.arg_fmt(), i64, String);

    if let Some(exp) = a_exp {
        let ecs = server.state.ecs_mut();
        let target = find_target(&ecs, a_alias, target);

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
            server.notify_client(client, msg);
        }
    }
}

fn handle_set_level(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let (a_lvl, a_alias) = scan_fmt_some!(&args, &action.arg_fmt(), u32, String);

    if let Some(lvl) = a_lvl {
        let target = find_target(&server.state.ecs(), a_alias, target);

        let mut error_msg = None;

        match target {
            Ok(player) => {
                let uid = server
                    .state
                    .ecs()
                    .read_storage::<Uid>()
                    .get(player)
                    .expect("Failed to get uid for player")
                    .0;
                server
                    .state
                    .notify_registered_clients(ServerMsg::PlayerListUpdate(
                        PlayerListUpdate::LevelChange(uid, lvl),
                    ));

                if let Some(stats) = server
                    .state
                    .ecs_mut()
                    .write_storage::<comp::Stats>()
                    .get_mut(player)
                {
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
            server.notify_client(client, msg);
        }
    }
}

use common::comp::Item;
fn handle_debug(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    if let Ok(items) = assets::load_glob::<Item>("common.items.debug.*") {
        server
            .state()
            .ecs()
            .write_storage::<comp::Inventory>()
            .get_mut(target)
            // TODO: Consider writing a `load_glob_cloned` in `assets` and using that here
            .map(|inv| inv.push_all_unique(items.iter().map(|item| item.as_ref().clone())));
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::InventoryUpdate>()
            .insert(
                target,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Debug),
            );
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(
                "Debug items not found? Something is very broken.",
            )),
        );
    }
}

fn handle_remove_lights(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let opt_radius = scan_fmt_some!(&args, &action.arg_fmt(), f32);
    let opt_player_pos = server.state.read_component_cloned::<comp::Pos>(target);
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
            client,
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
        client,
        ServerMsg::private(String::from(format!("Removed {} lights!", size))),
    );
}

fn handle_sudo(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let (Some(player_alias), Some(cmd), cmd_args) =
        scan_fmt_some!(&args, &action.arg_fmt(), String, String, String)
    {
        let cmd_args = cmd_args.unwrap_or(String::from(""));
        let cmd = if cmd.chars().next() == Some('/') {
            cmd.chars().skip(1).collect()
        } else {
            cmd
        };
        if let Some(action) = CHAT_COMMANDS.iter().find(|c| c.keyword() == cmd) {
            let ecs = server.state.ecs();
            let entity_opt = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
                .join()
                .find(|(_, player)| player.alias == player_alias)
                .map(|(entity, _)| entity);
            if let Some(entity) = entity_opt {
                get_handler(action)(server, client, entity, cmd_args, action);
            } else {
                server.notify_client(
                    client,
                    ServerMsg::private(format!("Could not find that player")),
                );
            }
        } else {
            server.notify_client(
                client,
                ServerMsg::private(format!("Unknown command: /{}", cmd)),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerMsg::private(String::from(action.help_string())),
        );
    }
}

fn handle_version(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    server.notify_client(
        client,
        ServerMsg::private(format!(
            "Server is running {}[{}]",
            common::util::GIT_HASH.to_string(),
            common::util::GIT_DATE.to_string(),
        )),
    );
}
