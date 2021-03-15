//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to
//! `CHAT_COMMANDS` and provide a handler function.

use crate::{
    settings::{BanRecord, EditableSetting},
    Server, SpawnPoint, StateExt,
};
use chrono::{NaiveTime, Timelike};
use common::{
    cmd::{ChatCommand, CHAT_COMMANDS, CHAT_SHORTCUTS},
    comp::{
        self,
        aura::{Aura, AuraKind, AuraTarget},
        buff::{BuffCategory, BuffData, BuffKind, BuffSource},
        inventory::item::MaterialStatManifest,
        invite::InviteKind,
        ChatType, Inventory, Item, LightEmitter, WaypointArea,
    },
    effect::Effect,
    event::{EventBus, ServerEvent},
    npc::{self, get_npc_name},
    resources::TimeOfDay,
    terrain::{Block, BlockKind, SpriteKind, TerrainChunkSize},
    uid::Uid,
    vol::RectVolSize,
    Damage, DamageSource, Explosion, LoadoutBuilder, RadiusEffect,
};
use common_net::{
    msg::{DisconnectReason, Notification, PlayerListUpdate, ServerGeneral},
    sync::WorldSyncExt,
};
use rand::Rng;
use specs::{Builder, Entity as EcsEntity, Join, WorldExt};
use std::{convert::TryFrom, time::Duration};
use vek::*;
use world::util::Sampler;

use crate::{client::Client, login_provider::LoginProvider};
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
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("You don't have permission to use '/{}'.", self.keyword()),
                ),
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
        ChatCommand::Ban => handle_ban,
        ChatCommand::Build => handle_build,
        ChatCommand::Campfire => handle_spawn_campfire,
        ChatCommand::Debug => handle_debug,
        ChatCommand::DebugColumn => handle_debug_column,
        ChatCommand::DropAll => handle_drop_all,
        ChatCommand::Dummy => handle_spawn_training_dummy,
        ChatCommand::Explosion => handle_explosion,
        ChatCommand::Faction => handle_faction,
        ChatCommand::GiveItem => handle_give_item,
        ChatCommand::Goto => handle_goto,
        ChatCommand::Group => handle_group,
        ChatCommand::GroupInvite => handle_group_invite,
        ChatCommand::GroupKick => handle_group_kick,
        ChatCommand::GroupLeave => handle_group_leave,
        ChatCommand::GroupPromote => handle_group_promote,
        ChatCommand::Health => handle_health,
        ChatCommand::Help => handle_help,
        ChatCommand::Home => handle_home,
        ChatCommand::JoinFaction => handle_join_faction,
        ChatCommand::Jump => handle_jump,
        ChatCommand::Kick => handle_kick,
        ChatCommand::Kill => handle_kill,
        ChatCommand::KillNpcs => handle_kill_npcs,
        ChatCommand::Lantern => handle_lantern,
        ChatCommand::Light => handle_light,
        ChatCommand::MakeBlock => handle_make_block,
        ChatCommand::MakeSprite => handle_make_sprite,
        ChatCommand::Motd => handle_motd,
        ChatCommand::Object => handle_object,
        ChatCommand::Players => handle_players,
        ChatCommand::Region => handle_region,
        ChatCommand::RemoveLights => handle_remove_lights,
        ChatCommand::Safezone => handle_safezone,
        ChatCommand::Say => handle_say,
        ChatCommand::SetMotd => handle_set_motd,
        ChatCommand::SkillPoint => handle_skill_point,
        ChatCommand::Spawn => handle_spawn,
        ChatCommand::Sudo => handle_sudo,
        ChatCommand::Tell => handle_tell,
        ChatCommand::Time => handle_time,
        ChatCommand::Tp => handle_tp,
        ChatCommand::Unban => handle_unban,
        ChatCommand::Version => handle_version,
        ChatCommand::Waypoint => handle_waypoint,
        ChatCommand::Whitelist => handle_whitelist,
        ChatCommand::World => handle_world,
    }
}

fn handle_drop_all(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    let pos = server
        .state
        .ecs()
        .read_storage::<comp::Pos>()
        .get(client)
        .cloned();

    let mut items = Vec::new();
    if let Some(mut inventory) = server
        .state
        .ecs()
        .write_storage::<comp::Inventory>()
        .get_mut(client)
    {
        items = inventory.drain().collect();
    }

    let mut rng = rand::thread_rng();

    let pos = pos.expect("expected pos for entity using dropall command");
    for item in items {
        let vel = Vec3::new(rng.gen_range(-0.1..0.1), rng.gen_range(-0.1..0.1), 0.5);

        server
            .state
            .create_object(Default::default(), comp::object::Body::Pouch)
            .with(comp::Pos(Vec3::new(
                pos.0.x + rng.gen_range(5.0..10.0),
                pos.0.y + rng.gen_range(5.0..10.0),
                pos.0.z + 5.0,
            )))
            .with(item)
            .with(comp::Vel(vel))
            .build();
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
        if let Ok(item) = Item::new_from_asset(&item_name.replace('/', ".").replace("\\", ".")) {
            let mut item: Item = item;
            if let Ok(()) = item.set_amount(give_amount.min(2000)) {
                server
                    .state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(target)
                    .map(|mut inv| {
                        if inv.push(item).is_some() {
                            server.notify_client(
                                client,
                                ServerGeneral::server_msg(
                                    ChatType::CommandError,
                                    format!(
                                        "Player inventory full. Gave 0 of {} items.",
                                        give_amount
                                    ),
                                ),
                            );
                        }
                    });
            } else {
                let msm = server.state.ecs().read_resource::<MaterialStatManifest>();
                // This item can't stack. Give each item in a loop.
                server
                    .state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(target)
                    .map(|mut inv| {
                        for i in 0..give_amount {
                            if inv.push(item.duplicate(&msm)).is_some() {
                                server.notify_client(
                                    client,
                                    ServerGeneral::server_msg(
                                        ChatType::CommandError,
                                        format!(
                                            "Player inventory full. Gave {} of {} items.",
                                            i, give_amount
                                        ),
                                    ),
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
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Invalid item: {}", item_name),
                ),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}

fn handle_make_block(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Some(block_name) = scan_fmt_some!(&args, &action.arg_fmt(), String) {
        if let Ok(bk) = BlockKind::try_from(block_name.as_str()) {
            match server.state.read_component_copied::<comp::Pos>(target) {
                Some(pos) => server.state.set_block(
                    pos.0.map(|e| e.floor() as i32),
                    Block::new(bk, Rgb::broadcast(255)),
                ),
                None => server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandError,
                        String::from("You have no position."),
                    ),
                ),
            }
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Invalid block kind: {}", block_name),
                ),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}

fn handle_make_sprite(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Some(sprite_name) = scan_fmt_some!(&args, &action.arg_fmt(), String) {
        if let Ok(sk) = SpriteKind::try_from(sprite_name.as_str()) {
            match server.state.read_component_copied::<comp::Pos>(target) {
                Some(pos) => {
                    let pos = pos.0.map(|e| e.floor() as i32);
                    let new_block = server
                        .state
                        .get_block(pos)
                        // TODO: Make more principled.
                        .unwrap_or_else(|| Block::air(SpriteKind::Empty))
                        .with_sprite(sk);
                    server.state.set_block(pos, new_block);
                },
                None => server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandError,
                        String::from("You have no position."),
                    ),
                ),
            }
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Invalid sprite kind: {}", sprite_name),
                ),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
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
        ServerGeneral::server_msg(
            ChatType::CommandError,
            (*server.editable_settings().server_description).clone(),
        ),
    );
}

fn handle_set_motd(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let data_dir = server.data_dir();
    match scan_fmt!(&args, &action.arg_fmt(), String) {
        Ok(msg) => {
            server
                .editable_settings_mut()
                .server_description
                .edit(data_dir.as_ref(), |d| **d = msg.clone());
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Server description set to \"{}\"", msg),
                ),
            );
        },
        Err(_) => {
            server
                .editable_settings_mut()
                .server_description
                .edit(data_dir.as_ref(), |d| d.clear());
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    "Removed server description".to_string(),
                ),
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
        match server.state.read_component_copied::<comp::Pos>(target) {
            Some(current_pos) => {
                server
                    .state
                    .write_component(target, comp::Pos(current_pos.0 + Vec3::new(x, y, z)));
                server.state.write_component(target, comp::ForceUpdate);
            },
            None => server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, "You have no position."),
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
            .read_component_copied::<comp::Pos>(target)
            .is_some()
        {
            server
                .state
                .write_component(target, comp::Pos(Vec3::new(x, y, z)));
            server.state.write_component(target, comp::ForceUpdate);
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, "You have no position."),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}

fn handle_home(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    if server
        .state
        .read_component_copied::<comp::Pos>(target)
        .is_some()
    {
        let home_pos = server.state.ecs().read_resource::<SpawnPoint>().0;
        let time = *server
            .state
            .ecs()
            .read_resource::<common::resources::Time>();

        server.state.write_component(target, comp::Pos(home_pos));
        server
            .state
            .write_component(target, comp::Waypoint::temp_new(home_pos, time));
        server.state.write_component(target, comp::ForceUpdate);
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position."),
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
        comp::HealthSource::Damage {
            kind: DamageSource::Other,
            by: Some(*uid),
        }
    } else {
        comp::HealthSource::Command
    };
    server
        .state
        .ecs_mut()
        .write_storage::<comp::Health>()
        .get_mut(target)
        .map(|mut h| h.set_to(0, reason));
}

fn handle_time(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    const DAY: u64 = 86400;

    let time_in_seconds = server.state.ecs_mut().read_resource::<TimeOfDay>().0;
    let current_day = time_in_seconds as u64 / DAY;
    let day_start = (current_day * DAY) as f64;

    // Find the next occurence of the given time in the day/night cycle
    let next_cycle = |time| {
        let new_time = day_start + time;
        new_time
            + if new_time < time_in_seconds {
                DAY as f64
            } else {
                0.0
            }
    };

    let time = scan_fmt_some!(&args, &action.arg_fmt(), String);
    let new_time = match time.as_deref() {
        Some("midnight") => {
            next_cycle(NaiveTime::from_hms(0, 0, 0).num_seconds_from_midnight() as f64)
        },
        Some("night") => {
            next_cycle(NaiveTime::from_hms(20, 0, 0).num_seconds_from_midnight() as f64)
        },
        Some("dawn") => next_cycle(NaiveTime::from_hms(5, 0, 0).num_seconds_from_midnight() as f64),
        Some("morning") => {
            next_cycle(NaiveTime::from_hms(8, 0, 0).num_seconds_from_midnight() as f64)
        },
        Some("day") => next_cycle(NaiveTime::from_hms(10, 0, 0).num_seconds_from_midnight() as f64),
        Some("noon") => {
            next_cycle(NaiveTime::from_hms(12, 0, 0).num_seconds_from_midnight() as f64)
        },
        Some("dusk") => {
            next_cycle(NaiveTime::from_hms(17, 0, 0).num_seconds_from_midnight() as f64)
        },
        Some(n) => match n.parse() {
            Ok(n) => n,
            Err(_) => match NaiveTime::parse_from_str(n, "%H:%M") {
                // Relative to current day
                Ok(time) => next_cycle(time.num_seconds_from_midnight() as f64),
                // Accept `u12345`, seconds since midnight day 0
                Err(_) => match n
                    .get(1..)
                    .filter(|_| n.starts_with('u'))
                    .and_then(|n| n.trim_start_matches('u').parse::<u64>().ok())
                {
                    // Absolute time (i.e: since in-game epoch)
                    Some(n) => n as f64,
                    None => {
                        server.notify_client(
                            client,
                            ServerGeneral::server_msg(
                                ChatType::CommandError,
                                format!("'{}' is not a valid time.", n),
                            ),
                        );
                        return;
                    },
                },
            },
        },
        None => {
            // Would this ever change? Perhaps in a few hundred thousand years some
            // game archeologists of the future will resurrect the best game of all
            // time which, obviously, would be Veloren. By that time, the inescapable
            // laws of thermodynamics will mean that the earth's rotation period
            // would be slower. Of course, a few hundred thousand years is enough
            // for the circadian rhythm of human biology to have shifted to account
            // accordingly. When booting up Veloren for the first time in 337,241
            // years, they might feel a touch of anguish at the fact that their
            // earth days and the days within the game do not neatly divide into
            // one-another. Understandably, they'll want to change this. Who
            // wouldn't? It would be like turning the TV volume up to an odd number
            // or having a slightly untuned radio (assuming they haven't begun
            // broadcasting information directly into their brains). Totally
            // unacceptable. No, the correct and proper thing to do would be to
            // release a retroactive definitive edition DLC for $99 with the very
            // welcome addition of shorter day periods and a complementary
            // 'developer commentary' mode created by digging up the long-decayed
            // skeletons of the Veloren team, measuring various attributes of their
            // jawlines, and using them to recreate their voices. But how to go about
            // this Herculean task? This code is jibberish! The last of the core Rust
            // dev team died exactly 337,194 years ago! Rust is now a long-forgotten
            // dialect of the ancient ones, lost to the sands of time. Ashes to ashes,
            // dust to dust. When all hope is lost, one particularly intrepid
            // post-human hominid exployed by the 'Veloren Revival Corp' (no doubt we
            // still won't have gotted rid of this blasted 'capitalism' thing by then)
            // might notice, after years of searching, a particularly curious
            // inscription within the code. The letters `D`, `A`, `Y`. Curious! She
            // consults the post-human hominid scholars of the old. Care to empathise
            // with her shock when she discovers that these symbols, as alien as they
            // may seem, correspond exactly to the word `ⓕя𝐢ᵇᵇ𝔩Ｅ`, the word for
            // 'day' in the post-human hominid language, which is of course universal.
            // Imagine also her surprise when, after much further translating, she
            // finds a comment predicting her very existence and her struggle to
            // decode this great mystery. Rejoyce! The Veloren Revival Corp. may now
            // persist with their great Ultimate Edition DLC because the day period
            // might now be changed because they have found the constant that controls
            // it! Everybody was henceforth happy until the end of time.
            //
            // This one's for you, xMac ;)
            let current_time = NaiveTime::from_num_seconds_from_midnight_opt(
                // Wraps around back to 0s if it exceeds 24 hours (24 hours = 86400s)
                (time_in_seconds as u64 % DAY) as u32,
                0,
            );
            let msg = match current_time {
                Some(time) => format!("It is {}", time.format("%H:%M").to_string()),
                None => String::from("Unknown Time"),
            };
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandInfo, msg),
            );
            return;
        },
    };

    server.state.ecs_mut().write_resource::<TimeOfDay>().0 = new_time;

    if let Some(new_time) =
        NaiveTime::from_num_seconds_from_midnight_opt(((new_time as u64) % 86400) as u32, 0)
    {
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                format!("Time changed to: {}", new_time.format("%H:%M").to_string(),),
            ),
        );
    }
}

fn handle_health(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Ok(hp) = scan_fmt!(&args, &action.arg_fmt(), u32) {
        if let Some(mut health) = server
            .state
            .ecs()
            .write_storage::<comp::Health>()
            .get_mut(target)
        {
            health.set_to(hp * 10, comp::HealthSource::Command);
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, "You have no health."),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You must specify health amount!"),
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
    if client != target {
        // Notify target that an admin changed the alias due to /sudo
        server.notify_client(
            target,
            ServerGeneral::server_msg(ChatType::CommandInfo, "An admin changed your alias."),
        );
        return;
    }
    if let Ok(alias) = scan_fmt!(&args, &action.arg_fmt(), String) {
        if !comp::Player::alias_is_valid(&alias) {
            // Prevent silly aliases
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, "Invalid alias."),
            );
            return;
        }
        let old_alias_optional = server
            .state
            .ecs_mut()
            .write_storage::<comp::Player>()
            .get_mut(target)
            .map(|mut player| std::mem::replace(&mut player.alias, alias));

        // Update name on client player lists
        let ecs = server.state.ecs();
        if let (Some(uid), Some(player), Some(old_alias)) = (
            ecs.read_storage::<Uid>().get(target),
            ecs.read_storage::<comp::Player>().get(target),
            old_alias_optional,
        ) {
            let msg = ServerGeneral::PlayerListUpdate(PlayerListUpdate::Alias(
                *uid,
                player.alias.clone(),
            ));
            server.state.notify_players(msg);

            // Announce alias change if target has a Body.
            if ecs.read_storage::<comp::Body>().get(target).is_some() {
                server.state.notify_players(ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    format!("{} is now known as {}.", old_alias, player.alias),
                ));
            }
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
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
    } else if client != target {
        Some(client)
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You must specify a player name"),
        );
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
        return;
    };
    if let Some(_pos) = server.state.read_component_copied::<comp::Pos>(target) {
        if let Some(player) = opt_player {
            if let Some(pos) = server.state.read_component_copied::<comp::Pos>(player) {
                server.state.write_component(target, pos);
                server.state.write_component(target, comp::ForceUpdate);
            } else {
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandError,
                        "Unable to teleport to player!",
                    ),
                );
            }
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, "Player not found!"),
            );
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
        );
    }
}

fn handle_spawn(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    match scan_fmt_some!(
        &args,
        &action.arg_fmt(),
        String,
        npc::NpcBody,
        String,
        String
    ) {
        (Some(opt_align), Some(npc::NpcBody(id, mut body)), opt_amount, opt_ai) => {
            let uid = server
                .state
                .read_component_copied(target)
                .expect("Expected player to have a UID");
            if let Some(alignment) = parse_alignment(uid, &opt_align) {
                let amount = opt_amount
                    .and_then(|a| a.parse().ok())
                    .filter(|x| *x > 0)
                    .unwrap_or(1)
                    .min(50);

                let ai = opt_ai.unwrap_or_else(|| "true".to_string());

                match server.state.read_component_copied::<comp::Pos>(target) {
                    Some(pos) => {
                        let agent =
                            if let comp::Alignment::Owned(_) | comp::Alignment::Npc = alignment {
                                comp::Agent::default()
                            } else {
                                comp::Agent::default().with_patrol_origin(pos.0)
                            };

                        for _ in 0..amount {
                            let vel = Vec3::new(
                                rand::thread_rng().gen_range(-2.0..3.0),
                                rand::thread_rng().gen_range(-2.0..3.0),
                                10.0,
                            );

                            let body = body();

                            let loadout =
                                LoadoutBuilder::build_loadout(body, None, None, None).build();

                            let inventory = Inventory::new_with_loadout(loadout);

                            let mut entity_base = server
                                .state
                                .create_npc(
                                    pos,
                                    comp::Stats::new(get_npc_name(
                                        id,
                                        npc::BodyType::from_body(body),
                                    )),
                                    comp::Health::new(body, 1),
                                    comp::Poise::new(body),
                                    inventory,
                                    body,
                                )
                                .with(comp::Vel(vel))
                                .with(comp::MountState::Unmounted)
                                .with(alignment);

                            if ai == "true" {
                                entity_base = entity_base.with(agent.clone());
                            }

                            let new_entity = entity_base.build();

                            // Add to group system if a pet
                            if matches!(alignment, comp::Alignment::Owned { .. }) {
                                let state = server.state();
                                let clients = state.ecs().read_storage::<Client>();
                                let uids = state.ecs().read_storage::<Uid>();
                                let mut group_manager =
                                    state.ecs().write_resource::<comp::group::GroupManager>();
                                group_manager.new_pet(
                                    new_entity,
                                    target,
                                    &mut state.ecs().write_storage(),
                                    &state.ecs().entities(),
                                    &state.ecs().read_storage(),
                                    &uids,
                                    &mut |entity, group_change| {
                                        clients
                                            .get(entity)
                                            .and_then(|c| {
                                                group_change
                                                    .try_map(|e| uids.get(e).copied())
                                                    .map(|g| (g, c))
                                            })
                                            .map(|(g, c)| {
                                                c.send_fallible(ServerGeneral::GroupUpdate(g));
                                            });
                                    },
                                );
                            } else if let Some(group) = match alignment {
                                comp::Alignment::Wild => None,
                                comp::Alignment::Passive => None,
                                comp::Alignment::Enemy => Some(comp::group::ENEMY),
                                comp::Alignment::Npc | comp::Alignment::Tame => {
                                    Some(comp::group::NPC)
                                },
                                comp::Alignment::Owned(_) => unreachable!(),
                            } {
                                let _ =
                                    server.state.ecs().write_storage().insert(new_entity, group);
                            }

                            if let Some(uid) = server.state.ecs().uid_from_entity(new_entity) {
                                server.notify_client(
                                    client,
                                    ServerGeneral::server_msg(
                                        ChatType::CommandInfo,
                                        format!("Spawned entity with ID: {}", uid),
                                    ),
                                );
                            }
                        }
                        server.notify_client(
                            client,
                            ServerGeneral::server_msg(
                                ChatType::CommandInfo,
                                format!("Spawned {} entities", amount),
                            ),
                        );
                    },
                    None => server.notify_client(
                        client,
                        ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
                    ),
                }
            }
        },
        _ => {
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
            );
        },
    }
}

fn handle_spawn_training_dummy(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    match server.state.read_component_copied::<comp::Pos>(target) {
        Some(pos) => {
            let vel = Vec3::new(
                rand::thread_rng().gen_range(-2.0..3.0),
                rand::thread_rng().gen_range(-2.0..3.0),
                10.0,
            );

            let body = comp::Body::Object(comp::object::Body::TrainingDummy);

            let stats = comp::Stats::new("Training Dummy".to_string());

            let health = comp::Health::new(body, 0);
            let poise = comp::Poise::new(body);

            server
                .state
                .create_npc(pos, stats, health, poise, Inventory::new_empty(), body)
                .with(comp::Vel(vel))
                .with(comp::MountState::Unmounted)
                .build();

            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandInfo, "Spawned a training dummy"),
            );
        },
        None => server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
        ),
    }
}

fn handle_spawn_campfire(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    match server.state.read_component_copied::<comp::Pos>(target) {
        Some(pos) => {
            server
                .state
                .create_object(pos, comp::object::Body::CampfireLit)
                .with(LightEmitter {
                    col: Rgb::new(1.0, 0.65, 0.2),
                    strength: 2.0,
                    flicker: 1.0,
                    animated: true,
                })
                .with(WaypointArea::default())
                .with(comp::Auras::new(vec![Aura::new(
                    AuraKind::Buff {
                        kind: BuffKind::CampfireHeal,
                        data: BuffData::new(0.02, Some(Duration::from_secs(1))),
                        category: BuffCategory::Natural,
                        source: BuffSource::World,
                    },
                    5.0,
                    None,
                    AuraTarget::All,
                )]))
                .build();

            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandInfo, "Spawned a campfire"),
            );
        },
        None => server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
        ),
    }
}

fn handle_safezone(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let range = scan_fmt_some!(&args, &action.arg_fmt(), f32);

    match server.state.read_component_copied::<comp::Pos>(target) {
        Some(pos) => {
            server
                .state
                .create_object(pos, comp::object::Body::BoltNature)
                .with(comp::Mass(10_f32.powi(10)))
                .with(comp::Auras::new(vec![Aura::new(
                    AuraKind::Buff {
                        kind: BuffKind::Invulnerability,
                        data: BuffData::new(1.0, Some(Duration::from_secs(1))),
                        category: BuffCategory::Natural,
                        source: BuffSource::World,
                    },
                    range.unwrap_or(100.0),
                    None,
                    AuraTarget::All,
                )]))
                .build();

            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandInfo, "Spawned a safe zone"),
            );
        },
        None => server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
        ),
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
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            entity_tuples.join().fold(
                format!("{} online players:", entity_tuples.join().count()),
                |s, (_, player, stat)| format!("{}\n[{}]{}", s, player.alias, stat.name,),
            ),
        ),
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
            ServerGeneral::server_msg(ChatType::CommandInfo, "Toggled off build mode!"),
        );
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .insert(target, comp::CanBuild);
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, "Toggled on build mode!"),
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
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, cmd.help_string()),
        );
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
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, message),
        );
    }
}

fn parse_alignment(owner: Uid, alignment: &str) -> Option<comp::Alignment> {
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
    let mut healths = ecs.write_storage::<comp::Health>();
    let players = ecs.read_storage::<comp::Player>();
    let mut count = 0;
    for (mut health, ()) in (&mut healths, !&players).join() {
        count += 1;
        health.set_to(0, comp::HealthSource::Command);
    }
    let text = if count > 0 {
        format!("Destroyed {} NPCs.", count)
    } else {
        "No NPCs on server.".to_string()
    };
    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, text),
    );
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
                .with(
                    comp::Ori::from_unnormalized_vec(
                        // converts player orientation into a 90° rotation for the object by using
                        // the axis with the highest value
                        {
                            let look_dir = ori.look_dir();
                            look_dir.map(|e| {
                                if e.abs() == look_dir.map(|e| e.abs()).reduce_partial_max() {
                                    e
                                } else {
                                    0.0
                                }
                            })
                        },
                    )
                    .unwrap_or_default(),
                )
                .build();
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    format!("Spawned: {}", obj_str_res.unwrap_or("<Unknown object>")),
                ),
            );
        } else {
            return server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, "Object not found!"),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
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
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    "cr, cg and cb values mustn't be negative.",
                ),
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
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, "Spawned object."),
        );
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
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
        if let Some(mut light) = server
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
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        "You adjusted flame strength and color.",
                    ),
                );
            } else {
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        "You adjusted flame strength.",
                    ),
                );
            }
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, "Please equip a lantern first"),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
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
            ServerGeneral::server_msg(
                ChatType::CommandError,
                "Explosion power mustn't be more than 512.",
            ),
        );
        return;
    } else if power <= 0.0 {
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandError,
                "Explosion power must be more than 0.",
            ),
        );
        return;
    }

    let ecs = server.state.ecs();

    match server.state.read_component_copied::<comp::Pos>(target) {
        Some(pos) => {
            ecs.read_resource::<EventBus<ServerEvent>>()
                .emit_now(ServerEvent::Explosion {
                    pos: pos.0,
                    explosion: Explosion {
                        effects: vec![
                            RadiusEffect::Entity(Effect::Damage(Damage {
                                source: DamageSource::Explosion,
                                value: 100.0 * power,
                            })),
                            RadiusEffect::TerrainDestruction(power),
                        ],
                        radius: 3.0 * power,
                        reagent: None,
                    },
                    owner: ecs.read_storage::<Uid>().get(target).copied(),
                })
        },
        None => server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
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
    match server.state.read_component_copied::<comp::Pos>(target) {
        Some(pos) => {
            let time = server.state.ecs().read_resource();
            let _ = server
                .state
                .ecs()
                .write_storage::<comp::Waypoint>()
                .insert(target, comp::Waypoint::temp_new(pos.0, *time));
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandInfo, "Waypoint saved!"),
            );
            server.notify_client(
                client,
                ServerGeneral::Notification(Notification::WaypointSaved),
            );
        },
        None => server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position!"),
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
                    .read_component_copied::<comp::Admin>(player)
                    .is_some()
                {
                    ecs.write_storage::<comp::Admin>().remove(player);
                    false
                } else {
                    ecs.write_storage().insert(player, comp::Admin).is_ok()
                };
                // Update player list so the player shows up as admin in client chat.
                let msg = ServerGeneral::PlayerListUpdate(PlayerListUpdate::Admin(
                    *ecs.read_storage::<Uid>()
                        .get(player)
                        .expect("Player should have uid"),
                    is_admin,
                ));
                server.state.notify_players(msg);
            },
            None => {
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandError,
                        format!("Player '{}' not found!", alias),
                    ),
                );
            },
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
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
            ServerGeneral::server_msg(ChatType::CommandError, "It's rude to impersonate people"),
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
                    ServerGeneral::server_msg(ChatType::CommandError, "You can't /tell yourself."),
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
            server.notify_client(client, ServerGeneral::ChatMode(mode));
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Player '{}' not found!", alias),
                ),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
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
            ServerGeneral::server_msg(ChatType::CommandError, "It's rude to impersonate people"),
        );
        return;
    }
    let ecs = server.state.ecs();
    if let Some(comp::Faction(faction)) = ecs.read_storage().get(client) {
        let mode = comp::ChatMode::Faction(faction.to_string());
        let _ = ecs.write_storage().insert(client, mode.clone());
        if !msg.is_empty() {
            if let Some(uid) = ecs.read_storage().get(client) {
                server.state.send_chat(mode.new_message(*uid, msg));
            }
        }
        server.notify_client(client, ServerGeneral::ChatMode(mode));
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandError,
                "Please join a faction with /join_faction",
            ),
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
            ServerGeneral::server_msg(ChatType::CommandError, "It's rude to impersonate people"),
        );
        return;
    }
    let ecs = server.state.ecs();
    if let Some(group) = ecs.read_storage::<comp::Group>().get(client) {
        let mode = comp::ChatMode::Group(*group);
        let _ = ecs.write_storage().insert(client, mode.clone());
        if !msg.is_empty() {
            if let Some(uid) = ecs.read_storage().get(client) {
                server.state.send_chat(mode.new_message(*uid, msg));
            }
        }
        server.notify_client(client, ServerGeneral::ChatMode(mode));
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "Please create a group first"),
        );
    }
}

fn handle_group_invite(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Some(target_alias) = scan_fmt_some!(&args, &action.arg_fmt(), String) {
        let ecs = server.state.ecs();
        let target_player_opt = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == target_alias)
            .map(|(entity, _)| entity);

        if let Some(target_player) = target_player_opt {
            let uid = *ecs
                .read_storage::<Uid>()
                .get(target_player)
                .expect("Failed to get uid for player");

            ecs.read_resource::<EventBus<ServerEvent>>()
                .emit_now(ServerEvent::InitiateInvite(client, uid, InviteKind::Group));

            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    format!("Invited {} to the group.", target_alias),
                ),
            );
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Player with alias {} not found", target_alias),
                ),
            )
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}

fn handle_group_kick(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    // Checking if leader is already done in group_manip
    if let Some(target_alias) = scan_fmt_some!(&args, &action.arg_fmt(), String) {
        let ecs = server.state.ecs();
        let target_player_opt = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == target_alias)
            .map(|(entity, _)| entity);

        if let Some(target_player) = target_player_opt {
            let uid = *ecs
                .read_storage::<Uid>()
                .get(target_player)
                .expect("Failed to get uid for player");

            ecs.read_resource::<EventBus<ServerEvent>>()
                .emit_now(ServerEvent::GroupManip(client, comp::GroupManip::Kick(uid)));
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Player with alias {} not found", target_alias),
                ),
            )
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}

fn handle_group_leave(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    server
        .state
        .ecs()
        .read_resource::<EventBus<ServerEvent>>()
        .emit_now(ServerEvent::GroupManip(client, comp::GroupManip::Leave));
}

fn handle_group_promote(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    // Checking if leader is already done in group_manip
    if let Some(target_alias) = scan_fmt_some!(&args, &action.arg_fmt(), String) {
        let ecs = server.state.ecs();
        let target_player_opt = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == target_alias)
            .map(|(entity, _)| entity);

        if let Some(target_player) = target_player_opt {
            let uid = *ecs
                .read_storage::<Uid>()
                .get(target_player)
                .expect("Failed to get uid for player");

            ecs.read_resource::<EventBus<ServerEvent>>()
                .emit_now(ServerEvent::GroupManip(
                    client,
                    comp::GroupManip::AssignLeader(uid),
                ));
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Player with alias {} not found", target_alias),
                ),
            )
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
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
            ServerGeneral::server_msg(ChatType::CommandError, "It's rude to impersonate people"),
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
            server.state.send_chat(mode.new_message(*uid, msg));
        }
    }
    server.notify_client(client, ServerGeneral::ChatMode(mode));
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
            ServerGeneral::server_msg(ChatType::CommandError, "It's rude to impersonate people"),
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
            server.state.send_chat(mode.new_message(*uid, msg));
        }
    }
    server.notify_client(client, ServerGeneral::ChatMode(mode));
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
            ServerGeneral::server_msg(ChatType::CommandError, "It's rude to impersonate people"),
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
            server.state.send_chat(mode.new_message(*uid, msg));
        }
    }
    server.notify_client(client, ServerGeneral::ChatMode(mode));
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
            ServerGeneral::server_msg(ChatType::CommandError, "It's rude to impersonate people"),
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
        let (faction_leave, mode) = if let Ok(faction) = scan_fmt!(&args, &action.arg_fmt(), String)
        {
            let mode = comp::ChatMode::Faction(faction.clone());
            let _ = server
                .state
                .ecs()
                .write_storage()
                .insert(client, mode.clone());
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
            (faction_leave, mode)
        } else {
            let mode = comp::ChatMode::default();
            let _ = server
                .state
                .ecs()
                .write_storage()
                .insert(client, mode.clone());
            let faction_leave = server
                .state
                .ecs()
                .write_storage()
                .remove(client)
                .map(|comp::Faction(f)| f);
            (faction_leave, mode)
        };
        if let Some(faction) = faction_leave {
            server.state.send_chat(
                ChatType::FactionMeta(faction.clone())
                    .chat_msg(format!("[{}] left faction ({})", alias, faction)),
            );
        }
        server.notify_client(client, ServerGeneral::ChatMode(mode));
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "Could not find your player alias"),
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
        ServerGeneral::server_msg(
            ChatType::CommandError,
            "Unsupported without worldgen enabled",
        ),
    );
}

#[cfg(feature = "worldgen")]
fn handle_debug_column(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let sim = server.world.sim();
    let sampler = server.world.sample_columns();
    let mut wpos = Vec2::new(0, 0);
    if let Ok((x, y)) = scan_fmt!(&args, &action.arg_fmt(), i32, i32) {
        wpos = Vec2::new(x, y);
    } else {
        match server.state.read_component_copied::<comp::Pos>(target) {
            Some(pos) => wpos = pos.0.xy().map(|x| x as i32),
            None => server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    String::from("You have no position."),
                ),
            ),
        }
    }
    let msg_generator = || {
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
        let col = sampler.get((wpos, server.index.as_index_ref()))?;
        let gradient = sim.get_gradient_approx(chunk_pos)?;
        let downhill = chunk.downhill;
        let river = &chunk.river;
        let flux = chunk.flux;

        Some(format!(
            r#"wpos: {:?}
alt {:?} ({:?})
water_alt {:?} ({:?})
basement {:?}
river {:?}
gradient {:?}
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
            gradient,
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
    if let Some(s) = msg_generator() {
        server.notify_client(client, ServerGeneral::server_msg(ChatType::CommandInfo, s));
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, "Not a pregenerated chunk."),
        );
    }
}

fn find_target(
    ecs: &specs::World,
    opt_alias: Option<String>,
    fallback: EcsEntity,
) -> Result<EcsEntity, ServerGeneral> {
    if let Some(alias) = opt_alias {
        (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == alias)
            .map(|(entity, _)| entity)
            .ok_or_else(|| {
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Player '{}' not found!", alias),
                )
            })
    } else {
        Ok(fallback)
    }
}

fn handle_skill_point(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    let (a_skill_tree, a_sp, a_alias) =
        scan_fmt_some!(&args, &action.arg_fmt(), String, u16, String);

    if let (Some(skill_tree), Some(sp)) = (a_skill_tree, a_sp) {
        let target = find_target(&server.state.ecs(), a_alias, target);

        let mut error_msg = None;

        match target {
            Ok(player) => {
                if let Some(skill_tree) = parse_skill_tree(&skill_tree) {
                    if let Some(mut stats) = server
                        .state
                        .ecs_mut()
                        .write_storage::<comp::Stats>()
                        .get_mut(player)
                    {
                        stats.skill_set.add_skill_points(skill_tree, sp);
                    } else {
                        error_msg = Some(ServerGeneral::server_msg(
                            ChatType::CommandError,
                            "Player has no stats!",
                        ));
                    }
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

fn parse_skill_tree(skill_tree: &str) -> Option<comp::skills::SkillGroupKind> {
    use comp::{item::tool::ToolKind, skills::SkillGroupKind};
    match skill_tree {
        "general" => Some(SkillGroupKind::General),
        "sword" => Some(SkillGroupKind::Weapon(ToolKind::Sword)),
        "axe" => Some(SkillGroupKind::Weapon(ToolKind::Axe)),
        "hammer" => Some(SkillGroupKind::Weapon(ToolKind::Hammer)),
        "bow" => Some(SkillGroupKind::Weapon(ToolKind::Bow)),
        "staff" => Some(SkillGroupKind::Weapon(ToolKind::Staff)),
        "sceptre" => Some(SkillGroupKind::Weapon(ToolKind::Sceptre)),
        _ => None,
    }
}

fn handle_debug(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: String,
    _action: &ChatCommand,
) {
    if let Ok(items) = comp::Item::new_from_asset_glob("common.items.debug.*") {
        server
            .state()
            .ecs()
            .write_storage::<comp::Inventory>()
            .get_mut(target)
            .map(|mut inv| inv.push_all_unique(items.into_iter()));
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
            ServerGeneral::server_msg(
                ChatType::CommandError,
                "Debug items not found? Something is very broken.",
            ),
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
    let opt_player_pos = server.state.read_component_copied::<comp::Pos>(target);
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
            ServerGeneral::server_msg(ChatType::CommandError, "You have no position."),
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
        ServerGeneral::server_msg(ChatType::CommandError, format!("Removed {} lights!", size)),
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
        let cmd_args = cmd_args.unwrap_or_else(|| String::from(""));
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
                    ServerGeneral::server_msg(ChatType::CommandError, "Could not find that player"),
                );
            }
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Unknown command: /{}", cmd),
                ),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
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
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            format!(
                "Server is running {}[{}]",
                common::util::GIT_HASH.to_string(),
                common::util::GIT_DATE.to_string(),
            ),
        ),
    );
}

fn handle_whitelist(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Ok((whitelist_action, username)) = scan_fmt!(&args, &action.arg_fmt(), String, String) {
        let lookup_uuid = || {
            server
                .state
                .ecs()
                .read_resource::<LoginProvider>()
                .username_to_uuid(&username)
                .map_err(|_| {
                    server.notify_client(
                        client,
                        ServerGeneral::server_msg(
                            ChatType::CommandError,
                            format!("Unable to determine UUID for username \"{}\"", &username),
                        ),
                    )
                })
                .ok()
        };

        if whitelist_action.eq_ignore_ascii_case("add") {
            if let Some(uuid) = lookup_uuid() {
                server
                    .editable_settings_mut()
                    .whitelist
                    .edit(server.data_dir().as_ref(), |w| w.insert(uuid));
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        format!("\"{}\" added to whitelist", username),
                    ),
                );
            }
        } else if whitelist_action.eq_ignore_ascii_case("remove") {
            if let Some(uuid) = lookup_uuid() {
                server
                    .editable_settings_mut()
                    .whitelist
                    .edit(server.data_dir().as_ref(), |w| w.remove(&uuid));
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        format!("\"{}\" removed from whitelist", username),
                    ),
                );
            }
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
            );
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}

fn kick_player(server: &mut Server, target_player: EcsEntity, reason: &str) {
    server
        .state
        .ecs()
        .read_resource::<EventBus<ServerEvent>>()
        .emit_now(ServerEvent::ClientDisconnect(target_player));
    server.notify_client(
        target_player,
        ServerGeneral::Disconnect(DisconnectReason::Kicked(reason.to_string())),
    );
}

fn handle_kick(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let (Some(target_alias), reason_opt) =
        scan_fmt_some!(&args, &action.arg_fmt(), String, String)
    {
        let reason = reason_opt.unwrap_or_default();
        let ecs = server.state.ecs();
        let target_player_opt = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
            .join()
            .find(|(_, player)| player.alias == target_alias)
            .map(|(entity, _)| entity);

        if let Some(target_player) = target_player_opt {
            kick_player(server, target_player, &reason);
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    format!(
                        "Kicked {} from the server with reason: {}",
                        target_alias, reason
                    ),
                ),
            );
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Player with alias {} not found", target_alias),
                ),
            )
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}

fn handle_ban(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let (Some(target_alias), reason_opt) =
        scan_fmt_some!(&args, &action.arg_fmt(), String, String)
    {
        let reason = reason_opt.unwrap_or_default();
        let uuid_result = server
            .state
            .ecs()
            .read_resource::<LoginProvider>()
            .username_to_uuid(&target_alias);

        if let Ok(uuid) = uuid_result {
            if server.editable_settings().banlist.contains_key(&uuid) {
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandError,
                        format!("{} is already on the banlist", target_alias),
                    ),
                )
            } else {
                server
                    .editable_settings_mut()
                    .banlist
                    .edit(server.data_dir().as_ref(), |b| {
                        b.insert(uuid, BanRecord {
                            username_when_banned: target_alias.clone(),
                            reason: reason.clone(),
                        });
                    });
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        format!(
                            "Added {} to the banlist with reason: {}",
                            target_alias, reason
                        ),
                    ),
                );

                // If the player is online kick them
                let ecs = server.state.ecs();
                let target_player_opt = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
                    .join()
                    .find(|(_, player)| player.alias == target_alias)
                    .map(|(entity, _)| entity);
                if let Some(target_player) = target_player_opt {
                    kick_player(server, target_player, &reason);
                }
            }
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Unable to determine UUID for username \"{}\"", target_alias),
                ),
            )
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}

fn handle_unban(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: String,
    action: &ChatCommand,
) {
    if let Ok(username) = scan_fmt!(&args, &action.arg_fmt(), String) {
        let uuid_result = server
            .state
            .ecs()
            .read_resource::<LoginProvider>()
            .username_to_uuid(&username);

        if let Ok(uuid) = uuid_result {
            server
                .editable_settings_mut()
                .banlist
                .edit(server.data_dir().as_ref(), |b| {
                    b.remove(&uuid);
                });
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    format!("{} was successfully unbanned", username),
                ),
            );
        } else {
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("Unable to determine UUID for username \"{}\"", username),
                ),
            )
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandError, action.help_string()),
        );
    }
}
