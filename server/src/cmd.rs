//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to
//! `CHAT_COMMANDS` and provide a handler function.

use crate::{Server, StateExt};
use chrono::{NaiveTime, Timelike};
use common::{
    assets,
    cmd::{ChatCommand, CHAT_COMMANDS, CHAT_SHORTCUTS},
    comp::{self, ChatType, Item},
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

use scan_fmt::{scan_fmt, scan_fmt_some};
use tracing::error;

pub trait ChatCommandExt {
    fn execute(&self, server: &mut Server, entity: EcsEntity, args: String);
}
impl ChatCommandExt for ChatCommand {
    #[allow(clippy::needless_return)] // TODO: Pending review in #587
    fn execute(&self, server: &mut Server, entity: EcsEntity, args: String) {
        if self.needs_admin() && !server.entity_is_admin(entity) {
            server.notify_client(
                entity,
                ChatType::CommandError.server_msg(format!(
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
        ChatCommand::Faction => handle_faction,
        ChatCommand::GiveExp => handle_give_exp,
        ChatCommand::GiveItem => handle_give_item,
        ChatCommand::Goto => handle_goto,
        ChatCommand::Group => handle_group,
        ChatCommand::Health => handle_health,
        ChatCommand::Help => handle_help,
        ChatCommand::JoinFaction => handle_join_faction,
        ChatCommand::JoinGroup => handle_join_group,
        ChatCommand::Jump => handle_jump,
        ChatCommand::Kill => handle_kill,
        ChatCommand::KillNpcs => handle_kill_npcs,
        ChatCommand::Lantern => handle_lantern,
        ChatCommand::Light => handle_light,
        ChatCommand::Motd => handle_motd,
        ChatCommand::Object => handle_object,
        ChatCommand::Players => handle_players,
        ChatCommand::Region => handle_region,
        ChatCommand::RemoveLights => handle_remove_lights,
        ChatCommand::Say => handle_say,
        ChatCommand::SetLevel => handle_set_level,
        ChatCommand::SetMotd => handle_set_motd,
        ChatCommand::Spawn => handle_spawn,
        ChatCommand::Sudo => handle_sudo,
        ChatCommand::Tell => handle_tell,
        ChatCommand::Time => handle_time,
        ChatCommand::Tp => handle_tp,
        ChatCommand::Version => handle_version,
        ChatCommand::Waypoint => handle_waypoint,
        ChatCommand::World => handle_world,
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
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
                                ChatType::CommandError.server_msg(format!(
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
                                    ChatType::CommandError.server_msg(format!(
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
                ChatType::CommandError.server_msg(format!("Invalid item: {}", item_name)),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
        );
    }
}

fn handle_motd(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    server.notify_client(
        client,
        ServerMsg::broadcast(server.settings().server_description.clone()),
    );
}

fn handle_set_motd(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    match scan_fmt!(&args, &action.arg_fmt(), String) {
        Ok(msg) => {
            server
                .settings_mut()
                .edit(|s| s.server_description = msg.clone());
            server.notify_client(
                client,
                ServerMsg::private(format!("Server description set to \"{}\"", msg)),
            );
        },
        Err(_) => {
            server.settings_mut().edit(|s| s.server_description.clear());
            server.notify_client(
                client,
                ServerMsg::private("Removed server description".to_string()),
            );
        },
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
                ChatType::CommandError.server_msg("You have no position."),
            ),
        }
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
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
                ChatType::CommandError.server_msg("You have no position."),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
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
    } else if let Some(uid) = server.state.read_storage::<Uid>().get(client) {
        comp::HealthSource::Attack { by: *uid }
    } else {
        comp::HealthSource::Command
    };
    server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(target)
        .map(|s| s.health.set_to(0, reason));
}

#[allow(clippy::option_as_ref_deref)] // TODO: Pending review in #587
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
                        ChatType::CommandError.server_msg(format!("'{}' is not a valid time.", n)),
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
            server.notify_client(client, ChatType::CommandInfo.server_msg(msg));
            return;
        },
    };

    server.state.ecs_mut().write_resource::<TimeOfDay>().0 =
        new_time.num_seconds_from_midnight() as f64;

    server.notify_client(
        client,
        ChatType::CommandInfo.server_msg(format!(
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
                ChatType::CommandError.server_msg("You have no health."),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("You must specify health amount!"),
        );
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
fn handle_alias(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if client != target {
        // Notify target that an admin changed the alias due to /sudo
        server.notify_client(
            target,
            ChatType::CommandInfo.server_msg("An admin changed your alias."),
        );
        return;
    }
    if let Ok(alias) = scan_fmt!(&args, &action.arg_fmt(), String) {
        if !comp::Player::alias_is_valid(&alias) {
            // Prevent silly aliases
            server.notify_client(client, ChatType::CommandError.server_msg("Invalid alias."));
            return;
        }
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
                server.state.notify_registered_clients(
                    ChatType::CommandInfo
                        .server_msg(format!("{} is now known as {}.", old_alias, player.alias)),
                );
            }
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
        );
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
#[allow(clippy::useless_format)] // TODO: Pending review in #587
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
    } else if client != target {
        Some(client)
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("You must specify a player name"),
        );
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
        );
        return;
    };
    if let Some(_pos) = server.state.read_component_cloned::<comp::Pos>(target) {
        if let Some(player) = opt_player {
            if let Some(pos) = server.state.read_component_cloned::<comp::Pos>(player) {
                server.state.write_component(target, pos);
                server.state.write_component(target, comp::ForceUpdate);
            } else {
                server.notify_client(
                    client,
                    ChatType::CommandError.server_msg("Unable to teleport to player!"),
                );
            }
        } else {
            server.notify_client(
                client,
                ChatType::CommandError.server_msg("Player not found!"),
            );
            server.notify_client(
                client,
                ChatType::CommandError.server_msg(action.help_string()),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("You have no position!"),
        );
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
#[allow(clippy::redundant_clone)] // TODO: Pending review in #587
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
                                    ChatType::CommandInfo
                                        .server_msg(format!("Spawned entity with ID: {}", uid)),
                                );
                            }
                        }
                        server.notify_client(
                            client,
                            ChatType::CommandInfo
                                .server_msg(format!("Spawned {} entities", amount)),
                        );
                    },
                    None => server.notify_client(
                        client,
                        ChatType::CommandError.server_msg("You have no position!"),
                    ),
                }
            }
        },
        _ => {
            server.notify_client(
                client,
                ChatType::CommandError.server_msg(action.help_string()),
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
        ChatType::CommandInfo.server_msg(entity_tuples.join().fold(
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
            ChatType::CommandInfo.server_msg("Toggled off build mode!"),
        );
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .insert(target, comp::CanBuild);
        server.notify_client(
            client,
            ChatType::CommandInfo.server_msg("Toggled on build mode!"),
        );
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
fn handle_help(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Some(cmd) = scan_fmt_some!(&args, &action.arg_fmt(), ChatCommand) {
        server.notify_client(client, ChatType::CommandInfo.server_msg(cmd.help_string()));
    } else {
        let mut message = String::new();
        for cmd in CHAT_COMMANDS.iter() {
            if !cmd.needs_admin() || server.entity_is_admin(client) {
                message += &cmd.help_string();
                message += "\n";
            }
        }
        message += "Additionally, you can use the following shortcuts:";
        for (k, v) in CHAT_SHORTCUTS.iter() {
            message += &format!(" /{} => /{}", k, v.keyword());
        }
        server.notify_client(client, ChatType::CommandInfo.server_msg(message));
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
    server.notify_client(client, ChatType::CommandInfo.server_msg(text));
}

#[allow(clippy::float_cmp)] // TODO: Pending review in #587
#[allow(clippy::needless_return)] // TODO: Pending review in #587
#[allow(clippy::useless_format)] // TODO: Pending review in #587
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
                ChatType::CommandInfo.server_msg(format!(
                    "Spawned: {}",
                    obj_str_res.unwrap_or("<Unknown object>")
                )),
            );
        } else {
            return server.notify_client(
                client,
                ChatType::CommandError.server_msg("Object not found!"),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("You have no position!"),
        );
    }
}

#[allow(clippy::useless_format)] // TODO: Pending review in #587
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
                ChatType::CommandError.server_msg("cr, cg and cb values mustn't be negative."),
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
        server.notify_client(client, ChatType::CommandInfo.server_msg("Spawned object."));
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("You have no position!"),
        );
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
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
                    ChatType::CommandInfo.server_msg("You adjusted flame strength and color."),
                );
            } else {
                server.notify_client(
                    client,
                    ChatType::CommandInfo.server_msg("You adjusted flame strength."),
                );
            }
        } else {
            server.notify_client(
                client,
                ChatType::CommandError.server_msg("Please equip a lantern first"),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
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
            ChatType::CommandError.server_msg("Explosion power mustn't be more than 512."),
        );
        return;
    } else if power <= 0.0 {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("Explosion power must be more than 0."),
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
            ChatType::CommandError.server_msg("You have no position!"),
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
            server.notify_client(client, ChatType::CommandInfo.server_msg("Waypoint saved!"));
            server.notify_client(client, ServerMsg::Notification(Notification::WaypointSaved));
        },
        None => server.notify_client(
            client,
            ChatType::CommandError.server_msg("You have no position!"),
        ),
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
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
            .find(|(_, player)| alias == player.alias)
            .map(|(entity, _)| entity);
        match opt_player {
            Some(player) => {
                let is_admin = if server
                    .state
                    .read_component_cloned::<comp::Admin>(player)
                    .is_some()
                {
                    ecs.write_storage::<comp::Admin>().remove(player);
                    false
                } else {
                    ecs.write_storage().insert(player, comp::Admin).is_ok()
                };
                // Update player list so the player shows up as admin in client chat.
                let msg = ServerMsg::PlayerListUpdate(PlayerListUpdate::Admin(
                    *ecs.read_storage::<Uid>()
                        .get(player)
                        .expect("Player should have uid"),
                    is_admin,
                ));
                server.state.notify_registered_clients(msg);
            },
            None => {
                server.notify_client(
                    client,
                    ChatType::CommandError.server_msg(format!("Player '{}' not found!", alias)),
                );
            },
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
        );
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
#[allow(clippy::useless_format)] // TODO: Pending review in #587
fn handle_tell(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if client != target {
        // This happens when [ab]using /sudo
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("It's rude to impersonate people"),
        );
        return;
    }
    if let (Some(alias), message_opt) = scan_fmt_some!(&args, &action.arg_fmt(), String, String) {
        let ecs = server.state.ecs();
        if let Some(player) = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == alias)
            .map(|(entity, _)| entity)
        {
            if player == client {
                server.notify_client(
                    client,
                    ChatType::CommandError.server_msg("You can't /tell yourself."),
                );
                return;
            }
            let client_uid = *ecs
                .read_storage()
                .get(client)
                .expect("Player must have uid");
            let player_uid = *ecs
                .read_storage()
                .get(player)
                .expect("Player must have uid");
            let mode = comp::ChatMode::Tell(player_uid);
            let _ = server
                .state
                .ecs()
                .write_storage()
                .insert(client, mode.clone());
            let msg = message_opt.unwrap_or_else(|| format!("{} wants to talk to you.", alias));
            server.state.send_chat(mode.new_message(client_uid, msg));
        } else {
            server.notify_client(
                client,
                ChatType::CommandError.server_msg(format!("Player '{}' not found!", alias)),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
        );
    }
}

fn handle_faction(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    msg: String,
    _action: &ChatCommand,
) {
    if client != target {
        // This happens when [ab]using /sudo
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("It's rude to impersonate people"),
        );
        return;
    }
    let ecs = server.state.ecs();
    if let Some(comp::Faction(faction)) = ecs.read_storage().get(client) {
        let mode = comp::ChatMode::Faction(faction.to_string());
        let _ = ecs.write_storage().insert(client, mode.clone());
        if !msg.is_empty() {
            if let Some(uid) = ecs.read_storage().get(client) {
                server
                    .state
                    .send_chat(mode.new_message(*uid, msg));
            }
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("Please join a faction with /join_faction"),
        );
    }
}

fn handle_group(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    msg: String,
    _action: &ChatCommand,
) {
    if client != target {
        // This happens when [ab]using /sudo
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("It's rude to impersonate people"),
        );
        return;
    }
    let ecs = server.state.ecs();
    if let Some(comp::Group(group)) = ecs.read_storage().get(client) {
        let mode = comp::ChatMode::Group(group.to_string());
        let _ = ecs.write_storage().insert(client, mode.clone());
        if !msg.is_empty() {
            if let Some(uid) = ecs.read_storage().get(client) {
                server
                    .state
                    .send_chat(mode.new_message(*uid, msg));
            }
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("Please join a group with /join_group"),
        );
    }
}

fn handle_region(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    msg: String,
    _action: &ChatCommand,
) {
    if client != target {
        // This happens when [ab]using /sudo
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("It's rude to impersonate people"),
        );
        return;
    }
    let mode = comp::ChatMode::Region;
    let _ = server
        .state
        .ecs()
        .write_storage()
        .insert(client, mode.clone());
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(client) {
            server
                .state
                .send_chat(mode.new_message(*uid, msg));
        }
    }
}

fn handle_say(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    msg: String,
    _action: &ChatCommand,
) {
    if client != target {
        // This happens when [ab]using /sudo
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("It's rude to impersonate people"),
        );
        return;
    }
    let mode = comp::ChatMode::Say;
    let _ = server
        .state
        .ecs()
        .write_storage()
        .insert(client, mode.clone());
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(client) {
            server
                .state
                .send_chat(mode.new_message(*uid, msg));
        }
    }
}

fn handle_world(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    msg: String,
    _action: &ChatCommand,
) {
    if client != target {
        // This happens when [ab]using /sudo
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("It's rude to impersonate people"),
        );
        return;
    }
    let mode = comp::ChatMode::World;
    let _ = server
        .state
        .ecs()
        .write_storage()
        .insert(client, mode.clone());
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(client) {
            server
                .state
                .send_chat(mode.new_message(*uid, msg));
        }
    }
}

fn handle_join_faction(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if client != target {
        // This happens when [ab]using /sudo
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("It's rude to impersonate people"),
        );
        return;
    }
    if let Some(alias) = server
        .state
        .ecs()
        .read_storage::<comp::Player>()
        .get(target)
        .map(|player| player.alias.clone())
    {
        let faction_leave = if let Ok(faction) = scan_fmt!(&args, &action.arg_fmt(), String) {
            let mode = comp::ChatMode::Faction(faction.clone());
            let _ = server.state.ecs().write_storage().insert(client, mode);
            let faction_leave = server
                .state
                .ecs()
                .write_storage()
                .insert(client, comp::Faction(faction.clone()))
                .ok()
                .flatten()
                .map(|f| f.0);
            server.state.send_chat(
                ChatType::FactionMeta(faction.clone())
                    .chat_msg(format!("[{}] joined faction ({})", alias, faction)),
            );
            faction_leave
        } else {
            let mode = comp::ChatMode::default();
            let _ = server.state.ecs().write_storage().insert(client, mode);
            server
                .state
                .ecs()
                .write_storage()
                .remove(client)
                .map(|comp::Faction(f)| f)
        };
        if let Some(faction) = faction_leave {
            server.state.send_chat(
                ChatType::FactionMeta(faction.clone())
                    .chat_msg(format!("[{}] left faction ({})", alias, faction)),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("Could not find your player alias"),
        );
    }
}

fn handle_join_group(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if client != target {
        // This happens when [ab]using /sudo
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("It's rude to impersonate people"),
        );
        return;
    }
    if let Some(alias) = server
        .state
        .ecs()
        .read_storage::<comp::Player>()
        .get(target)
        .map(|player| player.alias.clone())
    {
        let group_leave = if let Ok(group) = scan_fmt!(&args, &action.arg_fmt(), String) {
            let mode = comp::ChatMode::Group(group.clone());
            let _ = server.state.ecs().write_storage().insert(client, mode);
            let group_leave = server
                .state
                .ecs()
                .write_storage()
                .insert(client, comp::Group(group.clone()))
                .ok()
                .flatten()
                .map(|f| f.0);
            server.state.send_chat(
                ChatType::GroupMeta(group.clone())
                    .chat_msg(format!("[{}] joined group ({})", alias, group)),
            );
            group_leave
        } else {
            let mode = comp::ChatMode::default();
            let _ = server.state.ecs().write_storage().insert(client, mode);
            server
                .state
                .ecs()
                .write_storage()
                .remove(client)
                .map(|comp::Group(f)| f)
        };
        if let Some(group) = group_leave {
            server.state.send_chat(
                ChatType::GroupMeta(group.clone())
                    .chat_msg(format!("[{}] left group ({})", alias, group)),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg("Could not find your player alias"),
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
        ChatType::CommandError.server_msg("Unsupported without worldgen enabled"),
    );
}

#[cfg(feature = "worldgen")]
#[allow(clippy::blacklisted_name)] // TODO: Pending review in #587
#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
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
            server.notify_client(client, ChatType::CommandInfo.server_msg(s));
        } else {
            server.notify_client(
                client,
                ChatType::CommandError.server_msg("Not a pregenerated chunk."),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
        );
    }
}

#[allow(clippy::or_fun_call)] // TODO: Pending review in #587
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
            .ok_or(ChatType::CommandError.server_msg(format!("Player '{}' not found!", alias)))
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
                    error_msg = Some(ChatType::CommandError.server_msg("Player has no stats!"));
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
                let uid = *server
                    .state
                    .ecs()
                    .read_storage::<Uid>()
                    .get(player)
                    .expect("Failed to get uid for player");
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
                    error_msg = Some(ChatType::CommandError.server_msg("Player has no stats!"));
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
            ChatType::CommandError.server_msg("Debug items not found? Something is very broken."),
        );
    }
}

#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
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
            ChatType::CommandError.server_msg("You have no position."),
        ),
    }

    let size = to_delete.len();

    for entity in to_delete {
        if let Err(e) = server.state.delete_entity_recorded(entity) {
            error!(?e, "Failed to delete light: {:?}", e);
        }
    }

    server.notify_client(
        client,
        ChatType::CommandError.server_msg(format!("Removed {} lights!", size)),
    );
}

#[allow(clippy::chars_next_cmp)] // TODO: Pending review in #587
#[allow(clippy::useless_conversion)] // TODO: Pending review in #587
#[allow(clippy::or_fun_call)] // TODO: Pending review in #587
#[allow(clippy::useless_format)] // TODO: Pending review in #587
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
        if let Ok(action) = cmd.parse() {
            let ecs = server.state.ecs();
            let entity_opt = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
                .join()
                .find(|(_, player)| player.alias == player_alias)
                .map(|(entity, _)| entity);
            if let Some(entity) = entity_opt {
                get_handler(&action)(server, client, entity, cmd_args, &action);
            } else {
                server.notify_client(
                    client,
                    ChatType::CommandError.server_msg("Could not find that player"),
                );
            }
        } else {
            server.notify_client(
                client,
                ChatType::CommandError.server_msg(format!("Unknown command: /{}", cmd)),
            );
        }
    } else {
        server.notify_client(
            client,
            ChatType::CommandError.server_msg(action.help_string()),
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
        ChatType::CommandInfo.server_msg(format!(
            "Server is running {}[{}]",
            common::util::GIT_HASH.to_string(),
            common::util::GIT_DATE.to_string(),
        )),
    );
}
