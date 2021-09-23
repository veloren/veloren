//! # Implementing new commands.
//! To implement a new command provide a handler function
//! in [do_command].

use crate::{
    client::Client,
    login_provider::LoginProvider,
    settings::{
        Ban, BanAction, BanInfo, EditableSetting, SettingError, WhitelistInfo, WhitelistRecord,
    },
    sys::terrain::NpcData,
    wiring,
    wiring::{Logic, OutputFormula},
    Server, Settings, SpawnPoint, StateExt,
};
use assets::AssetExt;
use authc::Uuid;
use chrono::{NaiveTime, Timelike, Utc};
use common::{
    assets,
    cmd::{
        ChatCommand, BUFF_PACK, BUFF_PARSER, ITEM_SPECS, KIT_MANIFEST_PATH, PRESET_MANIFEST_PATH,
    },
    comp::{
        self,
        aura::{Aura, AuraKind, AuraTarget},
        buff::{Buff, BuffCategory, BuffData, BuffKind, BuffSource},
        inventory::item::{tool::AbilityMap, MaterialStatManifest, Quality},
        invite::InviteKind,
        AdminRole, ChatType, Inventory, Item, LightEmitter, WaypointArea,
    },
    depot,
    effect::Effect,
    event::{EventBus, ServerEvent},
    generation::EntityInfo,
    npc::{self, get_npc_name},
    resources::{BattleMode, PlayerPhysicsSettings, Time, TimeOfDay},
    terrain::{Block, BlockKind, SpriteKind, TerrainChunkSize},
    uid::Uid,
    vol::RectVolSize,
    Damage, DamageKind, DamageSource, Explosion, LoadoutBuilder, RadiusEffect,
};
use common_net::{
    msg::{DisconnectReason, Notification, PlayerListUpdate, ServerGeneral},
    sync::WorldSyncExt,
};
use common_state::{BuildAreaError, BuildAreas};
use core::{cmp::Ordering, convert::TryFrom, time::Duration};
use hashbrown::{HashMap, HashSet};
use humantime::Duration as HumanDuration;
use rand::Rng;
use specs::{storage::StorageEntry, Builder, Entity as EcsEntity, Join, WorldExt};
use std::str::FromStr;
use vek::*;
use wiring::{Circuit, Wire, WiringAction, WiringActionEffect, WiringElement};
use world::util::Sampler;

use common::comp::Alignment;
use tracing::{error, info, warn};

pub trait ChatCommandExt {
    fn execute(&self, server: &mut Server, entity: EcsEntity, args: Vec<String>);
}
impl ChatCommandExt for ChatCommand {
    fn execute(&self, server: &mut Server, entity: EcsEntity, args: Vec<String>) {
        if let Err(err) = do_command(server, entity, entity, args, self) {
            server.notify_client(
                entity,
                ServerGeneral::server_msg(ChatType::CommandError, err),
            );
        }
    }
}

type CmdResult<T> = Result<T, String>;

/// Handler function called when the command is executed.
/// # Arguments
/// * `&mut Server` - the `Server` instance executing the command.
/// * `EcsEntity` - an `Entity` corresponding to the player that invoked the
///   command.
/// * `EcsEntity` - an `Entity` for the player on whom the command is invoked.
///   This differs from the previous argument when using /sudo
/// * `Vec<String>` - a `Vec<String>` containing the arguments of the command
///   after the keyword.
/// * `&ChatCommand` - the command to execute with the above arguments.
/// Handler functions must parse arguments from the the given `String`
/// (`parse_args!` exists for this purpose).
///
/// # Returns
///
/// A `Result` that is `Ok` if the command went smoothly, and `Err` if it
/// failed; on failure, the string is sent to the client who initiated the
/// command.
type CommandHandler =
    fn(&mut Server, EcsEntity, EcsEntity, Vec<String>, &ChatCommand) -> CmdResult<()>;

fn do_command(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    cmd: &ChatCommand,
) -> CmdResult<()> {
    // Make sure your role is at least high enough to execute this command.
    if cmd.needs_role() > server.entity_admin_role(client) {
        return Err(format!(
            "You don't have permission to use '/{}'.",
            cmd.keyword()
        ));
    }

    let handler: CommandHandler = match cmd {
        ChatCommand::Adminify => handle_adminify,
        ChatCommand::Airship => handle_spawn_airship,
        ChatCommand::Alias => handle_alias,
        ChatCommand::ApplyBuff => handle_apply_buff,
        ChatCommand::Ban => handle_ban,
        ChatCommand::BattleMode => handle_battlemode,
        ChatCommand::BattleModeForce => handle_battlemode_force,
        ChatCommand::Build => handle_build,
        ChatCommand::BuildAreaAdd => handle_build_area_add,
        ChatCommand::BuildAreaList => handle_build_area_list,
        ChatCommand::BuildAreaRemove => handle_build_area_remove,
        ChatCommand::Campfire => handle_spawn_campfire,
        ChatCommand::DebugColumn => handle_debug_column,
        ChatCommand::DisconnectAllPlayers => handle_disconnect_all_players,
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
        ChatCommand::Kit => handle_kit,
        ChatCommand::Lantern => handle_lantern,
        ChatCommand::Light => handle_light,
        ChatCommand::MakeBlock => handle_make_block,
        ChatCommand::MakeNpc => handle_make_npc,
        ChatCommand::MakeSprite => handle_make_sprite,
        ChatCommand::Motd => handle_motd,
        ChatCommand::Object => handle_object,
        ChatCommand::PermitBuild => handle_permit_build,
        ChatCommand::Players => handle_players,
        ChatCommand::Region => handle_region,
        ChatCommand::RemoveLights => handle_remove_lights,
        ChatCommand::RevokeBuild => handle_revoke_build,
        ChatCommand::RevokeBuildAll => handle_revoke_build_all,
        ChatCommand::Safezone => handle_safezone,
        ChatCommand::Say => handle_say,
        ChatCommand::ServerPhysics => handle_server_physics,
        ChatCommand::SetMotd => handle_set_motd,
        ChatCommand::Site => handle_site,
        ChatCommand::SkillPoint => handle_skill_point,
        ChatCommand::SkillPreset => handle_skill_preset,
        ChatCommand::Spawn => handle_spawn,
        ChatCommand::Sudo => handle_sudo,
        ChatCommand::Tell => handle_tell,
        ChatCommand::Time => handle_time,
        ChatCommand::Tp => handle_tp,
        ChatCommand::Unban => handle_unban,
        ChatCommand::Version => handle_version,
        ChatCommand::Waypoint => handle_waypoint,
        ChatCommand::Wiring => handle_spawn_wiring,
        ChatCommand::Whitelist => handle_whitelist,
        ChatCommand::World => handle_world,
    };

    handler(server, client, target, args, cmd)
}

// Fallibly get position of entity with the given descriptor (used for error
// message).
fn position(server: &Server, entity: EcsEntity, descriptor: &str) -> CmdResult<comp::Pos> {
    server
        .state
        .ecs()
        .read_storage::<comp::Pos>()
        .get(entity)
        .copied()
        .ok_or_else(|| format!("Cannot get position for {:?}!", descriptor))
}

fn position_mut<T>(
    server: &mut Server,
    entity: EcsEntity,
    descriptor: &str,
    f: impl for<'a> FnOnce(&'a mut comp::Pos) -> T,
) -> CmdResult<T> {
    let mut pos_storage = server.state.ecs_mut().write_storage::<comp::Pos>();
    pos_storage
        .get_mut(entity)
        .map(f)
        .ok_or_else(|| format!("Cannot get position for {:?}!", descriptor))
}

fn insert_or_replace_component<C: specs::Component>(
    server: &mut Server,
    entity: EcsEntity,
    component: C,
    descriptor: &str,
) -> CmdResult<()> {
    server
        .state
        .ecs_mut()
        .write_storage()
        .insert(entity, component)
        .and(Ok(()))
        .map_err(|_| format!("Entity {:?} is dead!", descriptor))
}

fn uuid(server: &Server, entity: EcsEntity, descriptor: &str) -> CmdResult<Uuid> {
    server
        .state
        .ecs()
        .read_storage::<comp::Player>()
        .get(entity)
        .map(|player| player.uuid())
        .ok_or_else(|| format!("Cannot get player information for {:?}", descriptor))
}

fn real_role(server: &Server, uuid: Uuid, descriptor: &str) -> CmdResult<comp::AdminRole> {
    server
        .editable_settings()
        .admins
        .get(&uuid)
        .map(|record| record.role.into())
        .ok_or_else(|| format!("Cannot get administrator roles for {:?} uuid", descriptor))
}

// Fallibly get uid of entity with the given descriptor (used for error
// message).
fn uid(server: &Server, target: EcsEntity, descriptor: &str) -> CmdResult<Uid> {
    server
        .state
        .ecs()
        .read_storage::<Uid>()
        .get(target)
        .copied()
        .ok_or_else(|| format!("Cannot get uid for {:?}", descriptor))
}

fn area(server: &mut Server, area_name: &str) -> CmdResult<depot::Id<vek::Aabb<i32>>> {
    server
        .state
        .mut_resource::<BuildAreas>()
        .area_names()
        .get(area_name)
        .copied()
        .ok_or_else(|| format!("Area name not found: {}", area_name))
}

// Prevent use through sudo.
fn no_sudo(client: EcsEntity, target: EcsEntity) -> CmdResult<()> {
    if client == target {
        Ok(())
    } else {
        // This happens when [ab]using /sudo
        Err("It's rude to impersonate people".into())
    }
}

/// Ensure that client role is above target role, for the purpose of performing
/// some (often permanent) administrative action on the target.  Note that this
/// function is *not* a replacement for actually verifying that the client
/// should be able to execute the command at all, which still needs to be
/// rechecked, nor does it guarantee that either the client or the target
/// actually have an entry in the admin settings file.
///
/// For our purposes, there are *two* roles--temporary role, and permanent role.
/// For the purpose of these checks, currently *any* permanent role overrides
/// *any* temporary role (this may change if more roles are added that aren't
/// moderator or administrator).  If the permanent roles match, the temporary
/// roles are used as a tiebreaker.  /adminify should ensure that no one's
/// temporary role can be different from their permanent role without someone
/// with a higher role than their permanent role allowing it, and only permanent
/// roles should be recorded in the settings files.
fn verify_above_role(
    server: &mut Server,
    (client, client_uuid): (EcsEntity, Uuid),
    (player, player_uuid): (EcsEntity, Uuid),
    reason: &str,
) -> CmdResult<()> {
    let client_temp = server.entity_admin_role(client);
    let client_perm = server
        .editable_settings()
        .admins
        .get(&client_uuid)
        .map(|record| record.role);

    let player_temp = server.entity_admin_role(player);
    let player_perm = server
        .editable_settings()
        .admins
        .get(&player_uuid)
        .map(|record| record.role);

    if client_perm > player_perm || client_perm == player_perm && client_temp > player_temp {
        Ok(())
    } else {
        Err(reason.into())
    }
}

fn find_alias(ecs: &specs::World, alias: &str) -> CmdResult<(EcsEntity, Uuid)> {
    (&ecs.entities(), &ecs.read_storage::<comp::Player>())
        .join()
        .find(|(_, player)| player.alias == alias)
        .map(|(entity, player)| (entity, player.uuid()))
        .ok_or_else(|| format!("Player {:?} not found!", alias))
}

fn find_uuid(ecs: &specs::World, uuid: Uuid) -> CmdResult<EcsEntity> {
    (&ecs.entities(), &ecs.read_storage::<comp::Player>())
        .join()
        .find(|(_, player)| player.uuid() == uuid)
        .map(|(entity, _)| entity)
        .ok_or_else(|| format!("Player with UUID {:?} not found!", uuid))
}

fn find_username(server: &mut Server, username: &str) -> CmdResult<Uuid> {
    server
        .state
        .mut_resource::<LoginProvider>()
        .username_to_uuid(username)
        .map_err(|_| format!("Unable to determine UUID for username {:?}", username))
}

/// NOTE: Intended to be run only on logged-in clients.
fn uuid_to_username(
    server: &mut Server,
    fallback_entity: EcsEntity,
    uuid: Uuid,
) -> CmdResult<String> {
    let make_err = || format!("Unable to determine username for UUID {:?}", uuid);
    let player_storage = server.state.ecs().read_storage::<comp::Player>();

    let fallback_alias = &player_storage
        .get(fallback_entity)
        .ok_or_else(make_err)?
        .alias;

    server
        .state
        .ecs()
        .read_resource::<LoginProvider>()
        .uuid_to_username(uuid, fallback_alias)
        .map_err(|_| make_err())
}

fn edit_setting_feedback<S: EditableSetting>(
    server: &mut Server,
    client: EcsEntity,
    result: Option<(String, Result<(), SettingError<S>>)>,
    failure: impl FnOnce() -> String,
) -> CmdResult<()> {
    let (info, result) = result.ok_or_else(failure)?;
    match result {
        Ok(()) => {
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandInfo, info),
            );
            Ok(())
        },
        Err(SettingError::Io(err)) => {
            warn!(
                ?err,
                "Failed to write settings file to disk, but succeeded in memory (success message: \
                 {})",
                info,
            );
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!(
                        "Failed to write settings file to disk, but succeeded in memory.\n
                            Error (storage): {:?}\n
                            Success (memory): {}",
                        err, info
                    ),
                ),
            );
            Ok(())
        },
        Err(SettingError::Integrity(err)) => Err(format!(
            "Encountered an error while validating the request: {:?}",
            err
        )),
    }
}

/// Parse a series of command arguments into values, including collecting all
/// trailing arguments.
macro_rules! parse_args {
    ($args:expr, $($t:ty),* $(, ..$tail:ty)? $(,)?) => {
        {
            let mut args = $args.into_iter();
            (
                $(args.next().and_then(|s| s.parse::<$t>().ok())),*
                $(, args.map(|s| s.to_string()).collect::<$tail>())?
            )
        }
    };
}

fn handle_drop_all(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;

    let mut items = Vec::new();
    if let Some(mut inventory) = server
        .state
        .ecs()
        .write_storage::<comp::Inventory>()
        .get_mut(target)
    {
        items = inventory.drain().collect();
    }

    let mut rng = rand::thread_rng();

    let item_to_place = items
        .into_iter()
        .filter(|i| !matches!(i.quality(), Quality::Debug));
    for item in item_to_place {
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

    Ok(())
}

fn handle_give_item(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(item_name), give_amount_opt) = parse_args!(args, String, u32) {
        let give_amount = give_amount_opt.unwrap_or(1);
        if let Ok(item) = Item::new_from_asset(&item_name.replace('/', ".").replace("\\", ".")) {
            let mut item: Item = item;
            let mut res = Ok(());

            const MAX_GIVE_AMOUNT: u32 = 2000;
            // Cap give_amount for non-stackable items
            let give_amount = if item.is_stackable() {
                give_amount
            } else {
                give_amount.min(MAX_GIVE_AMOUNT)
            };

            if let Ok(()) = item.set_amount(give_amount) {
                server
                    .state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(target)
                    .map(|mut inv| {
                        // NOTE: Deliberately ignores items that couldn't be pushed.
                        if inv.push(item).is_err() {
                            res = Err(format!(
                                "Player inventory full. Gave 0 of {} items.",
                                give_amount
                            ));
                        }
                    });
            } else {
                let ability_map = server.state.ecs().read_resource::<AbilityMap>();
                let msm = server.state.ecs().read_resource::<MaterialStatManifest>();
                // This item can't stack. Give each item in a loop.
                server
                    .state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(target)
                    .map(|mut inv| {
                        for i in 0..give_amount {
                            // NOTE: Deliberately ignores items that couldn't be pushed.
                            if inv.push(item.duplicate(&ability_map, &msm)).is_err() {
                                res = Err(format!(
                                    "Player inventory full. Gave {} of {} items.",
                                    i, give_amount
                                ));
                                break;
                            }
                        }
                    });
            }

            insert_or_replace_component(
                server,
                target,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Given),
                "target",
            )?;
            res
        } else {
            Err(format!("Invalid item: {}", item_name))
        }
    } else {
        Err(action.help_string())
    }
}

fn handle_make_block(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(block_name), r, g, b) = parse_args!(args, String, u8, u8, u8) {
        if let Ok(bk) = BlockKind::from_str(block_name.as_str()) {
            let pos = position(server, target, "target")?;
            let new_block = Block::new(bk, Rgb::new(r, g, b).map(|e| e.unwrap_or(255)));
            let pos = pos.0.map(|e| e.floor() as i32);
            server.state.set_block(pos, new_block);
            #[cfg(feature = "persistent_world")]
            if let Some(terrain_persistence) = server
                .state
                .ecs()
                .try_fetch_mut::<crate::TerrainPersistence>()
                .as_mut()
            {
                terrain_persistence.set_block(pos, new_block);
            }
            Ok(())
        } else {
            Err(format!("Invalid block kind: {}", block_name))
        }
    } else {
        Err(action.help_string())
    }
}

fn handle_make_npc(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    let (entity_config, number) = parse_args!(args, String, i8);

    let entity_config = entity_config.ok_or_else(|| action.help_string())?;
    let number = match number {
        Some(i8::MIN..=0) => {
            return Err("Number of entities should be at least 1".to_owned());
        },
        Some(50..=i8::MAX) => {
            return Err("Number of entities should be less than 50".to_owned());
        },
        Some(number) => number,
        None => 1,
    };

    let rng = &mut rand::thread_rng();
    for _ in 0..number {
        let comp::Pos(pos) = position(server, target, "target")?;
        let entity_info = EntityInfo::at(pos).with_asset_expect(&entity_config);
        match NpcData::from_entity_info(entity_info, rng) {
            NpcData::Waypoint(_) => {
                return Err("Waypoint spawning is not implemented".to_owned());
            },
            NpcData::Data {
                loadout,
                pos,
                stats,
                skill_set,
                poise,
                health,
                body,
                agent,
                alignment,
                scale,
                drop_item,
            } => {
                let inventory = Inventory::new_with_loadout(loadout);

                let mut entity_builder = server
                    .state
                    .create_npc(pos, stats, skill_set, health, poise, inventory, body)
                    .with(alignment)
                    .with(scale)
                    .with(comp::Vel(Vec3::new(0.0, 0.0, 0.0)))
                    .with(comp::MountState::Unmounted);

                if let Some(agent) = agent {
                    entity_builder = entity_builder.with(agent);
                }

                if let Some(drop_item) = drop_item {
                    entity_builder = entity_builder.with(comp::ItemDrop(drop_item));
                }

                // Some would say it's a hack, some would say it's incomplete
                // simulation. But this is what we do to avoid PvP between npc.
                let npc_group = match alignment {
                    Alignment::Enemy => Some(comp::group::ENEMY),
                    Alignment::Npc | Alignment::Tame => Some(comp::group::NPC),
                    Alignment::Wild | Alignment::Passive | Alignment::Owned(_) => None,
                };
                if let Some(group) = npc_group {
                    entity_builder = entity_builder.with(group);
                }
                entity_builder.build();
            },
        };
    }

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            format!("Spawned {} entities from config: {}", number, entity_config),
        ),
    );

    Ok(())
}

fn handle_make_sprite(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(sprite_name) = parse_args!(args, String) {
        if let Ok(sk) = SpriteKind::try_from(sprite_name.as_str()) {
            let pos = position(server, target, "target")?;
            let pos = pos.0.map(|e| e.floor() as i32);
            let new_block = server
                .state
                .get_block(pos)
                // TODO: Make more principled.
                .unwrap_or_else(|| Block::air(SpriteKind::Empty))
                .with_sprite(sk);
            server.state.set_block(pos, new_block);
            #[cfg(feature = "persistent_world")]
            if let Some(terrain_persistence) = server
                .state
                .ecs()
                .try_fetch_mut::<crate::TerrainPersistence>()
                .as_mut()
            {
                terrain_persistence.set_block(pos, new_block);
            }
            Ok(())
        } else {
            Err(format!("Invalid sprite kind: {}", sprite_name))
        }
    } else {
        Err(action.help_string())
    }
}

fn handle_motd(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            (*server.editable_settings().server_description).clone(),
        ),
    );
    Ok(())
}

fn handle_set_motd(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let data_dir = server.data_dir();
    let client_uuid = uuid(server, client, "client")?;
    // Ensure the person setting this has a real role in the settings file, since
    // it's persistent.
    let _client_real_role = real_role(server, client_uuid, "client")?;
    match parse_args!(args, String) {
        Some(msg) => {
            let edit =
                server
                    .editable_settings_mut()
                    .server_description
                    .edit(data_dir.as_ref(), |d| {
                        let info = format!("Server description set to {:?}", msg);
                        **d = msg;
                        Some(info)
                    });
            drop(data_dir);
            edit_setting_feedback(server, client, edit, || {
                unreachable!("edit always returns Some")
            })
        },
        None => {
            let edit =
                server
                    .editable_settings_mut()
                    .server_description
                    .edit(data_dir.as_ref(), |d| {
                        d.clear();
                        Some("Removed server description".to_string())
                    });
            drop(data_dir);
            edit_setting_feedback(server, client, edit, || {
                unreachable!("edit always returns Some")
            })
        },
    }
}

fn handle_jump(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(x), Some(y), Some(z)) = parse_args!(args, f32, f32, f32) {
        position_mut(server, target, "target", |current_pos| {
            current_pos.0 += Vec3::new(x, y, z)
        })?;
        insert_or_replace_component(server, target, comp::ForceUpdate, "target")
    } else {
        Err(action.help_string())
    }
}

fn handle_goto(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(x), Some(y), Some(z)) = parse_args!(args, f32, f32, f32) {
        position_mut(server, target, "target", |current_pos| {
            current_pos.0 = Vec3::new(x, y, z)
        })?;
        insert_or_replace_component(server, target, comp::ForceUpdate, "target")
    } else {
        Err(action.help_string())
    }
}

/// TODO: Add autocompletion if possible (might require modifying enum to handle
/// dynamic values).
fn handle_site(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    #[cfg(feature = "worldgen")]
    if let Some(dest_name) = parse_args!(args, String) {
        let site = server
            .world
            .civs()
            .sites()
            .find(|site| {
                site.site_tmp
                    .map_or(false, |id| server.index.sites[id].name() == dest_name)
            })
            .ok_or_else(|| "Site not found".to_string())?;

        let site_pos = server.world.find_accessible_pos(
            server.index.as_index_ref(),
            TerrainChunkSize::center_wpos(site.center),
            false,
        );

        position_mut(server, target, "target", |current_pos| {
            current_pos.0 = site_pos
        })?;
        insert_or_replace_component(server, target, comp::ForceUpdate, "target")
    } else {
        Err(action.help_string())
    }

    #[cfg(not(feature = "worldgen"))]
    Ok(())
}

fn handle_home(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let home_pos = server.state.mut_resource::<SpawnPoint>().0;
    let time = *server.state.mut_resource::<common::resources::Time>();

    position_mut(server, target, "target", |current_pos| {
        current_pos.0 = home_pos
    })?;
    insert_or_replace_component(
        server,
        target,
        comp::Waypoint::temp_new(home_pos, time),
        "target",
    )?;
    insert_or_replace_component(server, target, comp::ForceUpdate, "target")
}

fn handle_kill(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    server
        .state
        .ecs_mut()
        .write_storage::<comp::Health>()
        .get_mut(target)
        .map(|mut h| h.kill());
    Ok(())
}

fn handle_time(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    const DAY: u64 = 86400;

    let time_in_seconds = server.state.mut_resource::<TimeOfDay>().0;
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

    let time = parse_args!(args, String);
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
                        return Err(format!("{:?} is not a valid time.", n));
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
            return Ok(());
        },
    };

    server.state.mut_resource::<TimeOfDay>().0 = new_time;

    // Update all clients with the new TimeOfDay (without this they would have to
    // wait for the next 100th tick to receive the update).
    let mut tod_lazymsg = None;
    let clients = server.state.ecs().read_storage::<Client>();
    for client in (&clients).join() {
        let msg = tod_lazymsg
            .unwrap_or_else(|| client.prepare(ServerGeneral::TimeOfDay(TimeOfDay(new_time))));
        let _ = client.send_prepared(&msg);
        tod_lazymsg = Some(msg);
    }

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
    Ok(())
}

fn handle_health(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(hp) = parse_args!(args, f32) {
        if let Some(mut health) = server
            .state
            .ecs()
            .write_storage::<comp::Health>()
            .get_mut(target)
        {
            let change = comp::HealthChange {
                amount: hp - health.current(),
                by: None,
                cause: None,
            };
            health.change_by(change);
            Ok(())
        } else {
            Err("You have no health".into())
        }
    } else {
        Err("You must specify health amount!".into())
    }
}

fn handle_alias(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(alias) = parse_args!(args, String) {
        // Prevent silly aliases
        comp::Player::alias_validate(&alias).map_err(|e| e.to_string())?;

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
        if client != target {
            // Notify target that an admin changed the alias due to /sudo
            server.notify_client(
                target,
                ServerGeneral::server_msg(ChatType::CommandInfo, "An admin changed your alias."),
            );
        }
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_tp(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    let player = if let Some(alias) = parse_args!(args, String) {
        find_alias(server.state.ecs(), &alias)?.0
    } else if client != target {
        client
    } else {
        return Err(action.help_string());
    };
    let player_pos = position(server, player, "player")?;
    position_mut(server, target, "target", |target_pos| {
        *target_pos = player_pos
    })?;
    insert_or_replace_component(server, target, comp::ForceUpdate, "target")
}

fn handle_spawn(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    match parse_args!(args, String, npc::NpcBody, u32, bool) {
        (Some(opt_align), Some(npc::NpcBody(id, mut body)), opt_amount, opt_ai) => {
            let uid = uid(server, target, "target")?;
            let alignment = parse_alignment(uid, &opt_align)?;
            let amount = opt_amount.filter(|x| *x > 0).unwrap_or(1).min(50);

            let ai = opt_ai.unwrap_or(true);
            let pos = position(server, target, "target")?;
            let mut agent = comp::Agent::from_body(&body());

            // If unowned, the agent should stay in a particular place
            if !matches!(alignment, comp::Alignment::Owned(_)) {
                agent = agent.with_patrol_origin(pos.0);
            }

            for _ in 0..amount {
                let vel = Vec3::new(
                    rand::thread_rng().gen_range(-2.0..3.0),
                    rand::thread_rng().gen_range(-2.0..3.0),
                    10.0,
                );

                let body = body();
                let loadout = LoadoutBuilder::from_default(&body).build();
                let inventory = Inventory::new_with_loadout(loadout);

                let mut entity_base = server
                    .state
                    .create_npc(
                        pos,
                        comp::Stats::new(get_npc_name(id, npc::BodyType::from_body(body))),
                        comp::SkillSet::default(),
                        Some(comp::Health::new(body, 1)),
                        comp::Poise::new(body),
                        inventory,
                        body,
                    )
                    .with(comp::Vel(vel))
                    .with(comp::MountState::Unmounted)
                    .with(alignment);

                if ai {
                    entity_base = entity_base.with(agent.clone());
                }

                let new_entity = entity_base.build();

                // Add to group system if a pet
                if matches!(alignment, comp::Alignment::Owned { .. }) {
                    let server_eventbus =
                        server.state.ecs().read_resource::<EventBus<ServerEvent>>();
                    server_eventbus.emit_now(ServerEvent::TamePet {
                        owner_entity: target,
                        pet_entity: new_entity,
                    });
                } else if let Some(group) = match alignment {
                    comp::Alignment::Wild => None,
                    comp::Alignment::Passive => None,
                    comp::Alignment::Enemy => Some(comp::group::ENEMY),
                    comp::Alignment::Npc | comp::Alignment::Tame => Some(comp::group::NPC),
                    comp::Alignment::Owned(_) => unreachable!(),
                } {
                    insert_or_replace_component(server, new_entity, group, "new entity")?;
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
            Ok(())
        },
        _ => Err(action.help_string()),
    }
}

fn handle_spawn_training_dummy(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;
    let vel = Vec3::new(
        rand::thread_rng().gen_range(-2.0..3.0),
        rand::thread_rng().gen_range(-2.0..3.0),
        10.0,
    );

    let body = comp::Body::Object(comp::object::Body::TrainingDummy);

    let stats = comp::Stats::new("Training Dummy".to_string());
    let skill_set = comp::SkillSet::default();
    let health = comp::Health::new(body, 0);
    let poise = comp::Poise::new(body);

    server
        .state
        .create_npc(
            pos,
            stats,
            skill_set,
            Some(health),
            poise,
            Inventory::new_empty(),
            body,
        )
        .with(comp::Vel(vel))
        .with(comp::MountState::Unmounted)
        .build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, "Spawned a training dummy"),
    );
    Ok(())
}

fn handle_spawn_airship(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let angle = parse_args!(args, f32);
    let mut pos = position(server, target, "target")?;
    pos.0.z += 50.0;
    const DESTINATION_RADIUS: f32 = 2000.0;
    let angle = angle.map(|a| a * std::f32::consts::PI / 180.0);
    let destination = angle.map(|a| {
        pos.0
            + Vec3::new(
                DESTINATION_RADIUS * a.cos(),
                DESTINATION_RADIUS * a.sin(),
                200.0,
            )
    });
    let ship = comp::ship::Body::random();
    let mut builder = server
        .state
        .create_ship(pos, ship, true)
        .with(LightEmitter {
            col: Rgb::new(1.0, 0.65, 0.2),
            strength: 2.0,
            flicker: 1.0,
            animated: true,
        });
    if let Some(pos) = destination {
        let (kp, ki, kd) = comp::agent::pid_coefficients(&comp::Body::Ship(ship));
        fn pure_z(sp: Vec3<f32>, pv: Vec3<f32>) -> f32 { (sp - pv).z }
        let agent = comp::Agent::from_body(&comp::Body::Ship(ship))
            .with_destination(pos)
            .with_position_pid_controller(comp::PidController::new(kp, ki, kd, pos, 0.0, pure_z));
        builder = builder.with(agent);
    }
    builder.build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, "Spawned an airship"),
    );
    Ok(())
}

fn handle_spawn_campfire(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;
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
        .with(comp::Auras::new(vec![
            Aura::new(
                AuraKind::Buff {
                    kind: BuffKind::CampfireHeal,
                    data: BuffData::new(0.02, Some(Duration::from_secs(1))),
                    category: BuffCategory::Natural,
                    source: BuffSource::World,
                },
                5.0,
                None,
                AuraTarget::All,
            ),
            Aura::new(
                AuraKind::Buff {
                    kind: BuffKind::Burning,
                    data: BuffData::new(2.0, Some(Duration::from_secs(10))),
                    category: BuffCategory::Natural,
                    source: BuffSource::World,
                },
                0.7,
                None,
                AuraTarget::All,
            ),
        ]))
        .build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, "Spawned a campfire"),
    );
    Ok(())
}

fn handle_safezone(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let range = parse_args!(args, f32);
    let pos = position(server, target, "target")?;
    server.state.create_safezone(range, pos).build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, "Spawned a safe zone"),
    );
    Ok(())
}

fn handle_permit_build(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(area_name) = parse_args!(args, String) {
        let bb_id = area(server, &area_name)?;
        let mut can_build = server.state.ecs().write_storage::<comp::CanBuild>();
        let entry = can_build
            .entry(target)
            .map_err(|_| "Cannot find target entity!".to_string())?;
        let mut comp_can_build = entry.or_insert(comp::CanBuild {
            enabled: false,
            build_areas: HashSet::new(),
        });
        comp_can_build.build_areas.insert(bb_id);
        drop(can_build);
        if client != target {
            server.notify_client(
                target,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    format!("You are now permitted to build in {}", area_name),
                ),
            );
        }
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                format!("Permission to build in {} granted", area_name),
            ),
        );
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_revoke_build(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(area_name) = parse_args!(args, String) {
        let bb_id = area(server, &area_name)?;
        let mut can_build = server.state.ecs_mut().write_storage::<comp::CanBuild>();
        if let Some(mut comp_can_build) = can_build.get_mut(target) {
            comp_can_build.build_areas.retain(|&x| x != bb_id);
            drop(can_build);
            if client != target {
                server.notify_client(
                    target,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        format!("Your permission to build in {} has been revoked", area_name),
                    ),
                );
            }
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    format!("Permission to build in {} revoked", area_name),
                ),
            );
            Ok(())
        } else {
            Err("You do not have permission to build.".into())
        }
    } else {
        Err(action.help_string())
    }
}

fn handle_revoke_build_all(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let ecs = server.state.ecs();

    ecs.write_storage::<comp::CanBuild>().remove(target);
    if client != target {
        server.notify_client(
            target,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                "Your build permissions have been revoked.",
            ),
        );
    }
    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, "All build permissions revoked"),
    );
    Ok(())
}

fn handle_players(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
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
    Ok(())
}

fn handle_build(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(mut can_build) = server
        .state
        .ecs()
        .write_storage::<comp::CanBuild>()
        .get_mut(target)
    {
        can_build.enabled ^= true;

        let toggle_string = if can_build.enabled { "on" } else { "off" };
        let msg = format!(
            "Toggled build mode {}.{}",
            toggle_string,
            if !can_build.enabled {
                ""
            } else if server.settings().experimental_terrain_persistence {
                " Experimental terrain persistence is enabled. The server will attempt to persist \
                 changes, but this is not guaranteed."
            } else {
                " Changes will not be persisted when a chunk unloads."
            },
        );

        let chat_msg = ServerGeneral::server_msg(ChatType::CommandInfo, msg);
        if client != target {
            server.notify_client(target, chat_msg.clone());
        }
        server.notify_client(client, chat_msg);
        Ok(())
    } else {
        Err("You do not have permission to build.".into())
    }
}

fn handle_build_area_add(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(area_name), Some(xlo), Some(xhi), Some(ylo), Some(yhi), Some(zlo), Some(zhi)) =
        parse_args!(args, String, i32, i32, i32, i32, i32, i32)
    {
        let build_areas = server.state.mut_resource::<BuildAreas>();
        let msg = ServerGeneral::server_msg(
            ChatType::CommandInfo,
            format!("Created build zone {}", area_name),
        );
        build_areas
            .insert(area_name, Aabb {
                min: Vec3::new(xlo, ylo, zlo),
                max: Vec3::new(xhi, yhi, zhi),
            })
            .map_err(|area_name| format!("Build zone {} already exists!", area_name))?;
        server.notify_client(client, msg);
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_build_area_list(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let build_areas = server.state.mut_resource::<BuildAreas>();
    let msg = ServerGeneral::server_msg(
        ChatType::CommandInfo,
        build_areas.area_names().iter().fold(
            "Build Areas:".to_string(),
            |acc, (area_name, bb_id)| {
                if let Some(aabb) = build_areas.areas().get(*bb_id) {
                    format!("{}\n{}: {} to {}", acc, area_name, aabb.min, aabb.max)
                } else {
                    acc
                }
            },
        ),
    );

    server.notify_client(client, msg);
    Ok(())
}

fn handle_build_area_remove(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(area_name) = parse_args!(args, String) {
        let build_areas = server.state.mut_resource::<BuildAreas>();

        build_areas.remove(&area_name).map_err(|err| match err {
            BuildAreaError::Reserved => format!(
                "Build area is reserved and cannot be removed: {}",
                area_name
            ),
            BuildAreaError::NotFound => format!("No such build area {}", area_name),
        })?;
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                format!("Removed build zone {}", area_name),
            ),
        );
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_help(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(cmd) = parse_args!(args, ChatCommand) {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, cmd.help_string()),
        )
    } else {
        let mut message = String::new();
        let entity_role = server.entity_admin_role(client);

        // Iterate through all commands you have permission to use.
        ChatCommand::iter()
            .filter(|cmd| cmd.needs_role() <= entity_role)
            .for_each(|cmd| {
                message += &cmd.help_string();
                message += "\n";
            });
        message += "Additionally, you can use the following shortcuts:";
        ChatCommand::iter()
            .filter_map(|cmd| cmd.short_keyword().map(|k| (k, cmd)))
            .for_each(|(k, cmd)| {
                message += &format!(" /{} => /{}", k, cmd.keyword());
            });

        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, message),
        )
    }
    Ok(())
}

fn parse_alignment(owner: Uid, alignment: &str) -> CmdResult<comp::Alignment> {
    match alignment {
        "wild" => Ok(comp::Alignment::Wild),
        "enemy" => Ok(comp::Alignment::Enemy),
        "npc" => Ok(comp::Alignment::Npc),
        "pet" => Ok(comp::Alignment::Owned(owner)),
        _ => Err(format!("Invalid alignment: {:?}", alignment)),
    }
}

fn handle_kill_npcs(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let kill_pets = if let Some(kill_option) = parse_args!(args, String) {
        kill_option.contains("--also-pets")
    } else {
        false
    };

    let ecs = server.state.ecs();
    let mut healths = ecs.write_storage::<comp::Health>();
    let players = ecs.read_storage::<comp::Player>();
    let alignments = ecs.read_storage::<comp::Alignment>();
    let mut count = 0;

    for (mut health, (), alignment) in (&mut healths, !&players, alignments.maybe()).join() {
        let should_kill = kill_pets
            || if let Some(Alignment::Owned(owned)) = alignment {
                ecs.entity_from_uid(owned.0)
                    .map_or(true, |owner| !players.contains(owner))
            } else {
                true
            };

        if should_kill {
            count += 1;
            health.kill();
        }
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

    Ok(())
}

fn handle_kit(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    use common::cmd::KitManifest;

    let notify = |server: &mut Server, kit_name: &str| {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, format!("Gave kit: {}", kit_name)),
        );
    };
    let name = parse_args!(args, String).ok_or_else(|| action.help_string())?;

    match name.as_str() {
        "all" => {
            // TODO: we will probably want to handle modular items here too
            let items = &ITEM_SPECS;
            let res = push_kit(
                items.iter().map(|item_id| (item_id.as_str(), 1)),
                items.len(),
                server,
                target,
            );
            if res.is_ok() {
                notify(server, "all");
            }
            res
        },
        kit_name => {
            let kits = KitManifest::load(KIT_MANIFEST_PATH)
                .map(|kits| kits.read())
                .map_err(|_| format!("Could not load manifest file {}", KIT_MANIFEST_PATH))?;

            let kit = kits
                .0
                .get(kit_name)
                .ok_or(format!("Kit '{}' not found", kit_name))?;

            let res = push_kit(
                kit.iter()
                    .map(|&(ref item_id, quantity)| (item_id.as_str(), quantity)),
                kit.len(),
                server,
                target,
            );
            if res.is_ok() {
                notify(server, kit_name);
            }
            res
        },
    }
}

fn push_kit<'a, I>(kit: I, count: usize, server: &mut Server, target: EcsEntity) -> CmdResult<()>
where
    I: Iterator<Item = (&'a str, u32)>,
{
    if let (Some(mut target_inventory), mut target_inv_update) = (
        server
            .state()
            .ecs()
            .write_storage::<comp::Inventory>()
            .get_mut(target),
        server.state.ecs().write_storage::<comp::InventoryUpdate>(),
    ) {
        // TODO: implement atomic `insert_all_or_nothing` on Inventory
        if target_inventory.free_slots() < count {
            return Err("Inventory doesn't have enough slots".to_owned());
        }
        for (item_id, quantity) in kit {
            let mut item = comp::Item::new_from_asset(item_id)
                .map_err(|_| format!("Unknown item: {}", item_id))?;
            let mut res = Ok(());

            // Either push stack or push one by one.
            if item.is_stackable() {
                // FIXME: in theory, this can fail,
                // but we don't have stack sizes yet.
                let _ = item.set_amount(quantity);
                res = target_inventory.push(item);
                let _ = target_inv_update.insert(
                    target,
                    comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Debug),
                );
            } else {
                let ability_map = server.state.ecs().read_resource::<AbilityMap>();
                let msm = server.state.ecs().read_resource::<MaterialStatManifest>();
                for _ in 0..quantity {
                    res = target_inventory.push(item.duplicate(&ability_map, &msm));
                    let _ = target_inv_update.insert(
                        target,
                        comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Debug),
                    );
                }
            }
            // I think it's possible to pick-up item during this loop
            // and fail into case where you had space but now you don't?
            if res.is_err() {
                return Err("Can't fit item to inventory".to_owned());
            }
        }
        Ok(())
    } else {
        Err("Could not get inventory".to_string())
    }
}

#[allow(clippy::float_cmp)] // TODO: Pending review in #587
fn handle_object(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let obj_type = parse_args!(args, String);

    let pos = position(server, target, "target")?;
    let ori = server
        .state
        .ecs()
        .read_storage::<comp::Ori>()
        .get(target)
        .copied()
        .ok_or_else(|| "Cannot get orientation for target".to_string())?;
    /*let builder = server.state
    .create_object(pos, ori, obj_type)
    .with(ori);*/
    let obj_str_res = obj_type.as_deref();
    if let Some(obj_type) = comp::object::ALL_OBJECTS
        .iter()
        .find(|o| Some(o.to_string()) == obj_str_res)
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
        Ok(())
    } else {
        Err("Object not found!".into())
    }
}

fn handle_light(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let (opt_r, opt_g, opt_b, opt_x, opt_y, opt_z, opt_s) =
        parse_args!(args, f32, f32, f32, f32, f32, f32, f32);

    let mut light_emitter = comp::LightEmitter::default();
    let mut light_offset_opt = None;

    if let (Some(r), Some(g), Some(b)) = (opt_r, opt_g, opt_b) {
        if r < 0.0 || g < 0.0 || b < 0.0 {
            return Err("cr, cg and cb values mustn't be negative.".into());
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
    let pos = position(server, target, "target")?;
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
    Ok(())
}

fn handle_lantern(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(s), r, g, b) = parse_args!(args, f32, f32, f32, f32) {
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
                )
            } else {
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        "You adjusted flame strength.",
                    ),
                )
            }
            Ok(())
        } else {
            Err("Please equip a lantern first".into())
        }
    } else {
        Err(action.help_string())
    }
}

fn handle_explosion(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let power = parse_args!(args, f32).unwrap_or(8.0);

    const MIN_POWER: f32 = 0.0;
    const MAX_POWER: f32 = 512.0;

    if power > MAX_POWER {
        return Err(format!(
            "Explosion power mustn't be more than {:?}.",
            MAX_POWER
        ));
    } else if power <= 0.0 {
        return Err(format!(
            "Explosion power must be more than {:?}.",
            MIN_POWER
        ));
    }

    let pos = position(server, target, "target")?;
    let owner = server
        .state
        .ecs()
        .read_storage::<Uid>()
        .get(target)
        .copied();
    server
        .state
        .mut_resource::<EventBus<ServerEvent>>()
        .emit_now(ServerEvent::Explosion {
            pos: pos.0,
            explosion: Explosion {
                effects: vec![
                    RadiusEffect::Entity(Effect::Damage(Damage {
                        source: DamageSource::Explosion,
                        kind: DamageKind::Energy,
                        value: 100.0 * power,
                    })),
                    RadiusEffect::TerrainDestruction(power),
                ],
                radius: 3.0 * power,
                reagent: None,
            },
            owner,
        });
    Ok(())
}

fn handle_waypoint(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;
    let time = *server.state.mut_resource::<common::resources::Time>();
    insert_or_replace_component(
        server,
        target,
        comp::Waypoint::temp_new(pos.0, time),
        "target",
    )?;
    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, "Waypoint saved!"),
    );
    server.notify_client(
        target,
        ServerGeneral::Notification(Notification::WaypointSaved),
    );
    Ok(())
}

fn handle_spawn_wiring(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    // Obviously it is a WIP - use it for debug

    let mut pos = position(server, target, "target")?;
    pos.0.x += 3.0;

    let mut outputs1 = HashMap::new();
    outputs1.insert(
        "deaths_last_tick".to_string(),
        wiring::OutputFormula::OnDeath {
            value: 1.0,
            radius: 30.0,
        },
    );
    outputs1.insert(
        "deaths_accumulated".to_string(),
        OutputFormula::Logic(Box::new(Logic {
            kind: wiring::LogicKind::Sum,
            left: OutputFormula::Logic(Box::new(Logic {
                kind: wiring::LogicKind::Sub,
                left: OutputFormula::Input {
                    name: "deaths_accumulated".to_string(),
                },
                right: OutputFormula::Logic(Box::new(Logic {
                    kind: wiring::LogicKind::Min,
                    left: OutputFormula::Input {
                        name: "pressed".to_string(),
                    },
                    right: OutputFormula::Input {
                        name: "deaths_accumulated".to_string(),
                    },
                })),
            })),
            right: OutputFormula::Input {
                name: "deaths_last_tick".to_string(),
            },
        })),
    );
    outputs1.insert("pressed".to_string(), OutputFormula::OnCollide {
        value: f32::MAX,
    });

    let builder1 = server
        .state
        .create_wiring(pos, comp::object::Body::Coins, WiringElement {
            actions: vec![WiringAction {
                formula: wiring::OutputFormula::Constant { value: 1.0 },
                threshold: 1.0,
                effects: vec![WiringActionEffect::SetLight {
                    r: wiring::OutputFormula::Input {
                        name: String::from("color"),
                    },
                    g: wiring::OutputFormula::Input {
                        name: String::from("color"),
                    },
                    b: wiring::OutputFormula::Input {
                        name: String::from("color"),
                    },
                }],
            }],
            inputs: HashMap::new(),
            outputs: outputs1,
        })
        .with(comp::Density(100_f32))
        .with(comp::Sticky);
    let ent1 = builder1.build();

    pos.0.x += 3.0;
    let builder2 = server
        .state
        .create_wiring(pos, comp::object::Body::Coins, WiringElement {
            actions: vec![
                WiringAction {
                    formula: wiring::OutputFormula::Input {
                        name: String::from("deaths_accumulated"),
                    },
                    threshold: 5.0,
                    effects: vec![WiringActionEffect::SpawnProjectile {
                        constr: comp::ProjectileConstructor::Arrow {
                            damage: 1.0,
                            energy_regen: 0.0,
                            knockback: 0.0,
                        },
                    }],
                },
                WiringAction {
                    formula: wiring::OutputFormula::Input {
                        name: String::from("deaths_accumulated"),
                    },
                    threshold: 1.0,
                    effects: vec![WiringActionEffect::SetBlock {
                        coords: vek::Vec3::new(0, 0, pos.0.z as i32),
                        block: Block::new(BlockKind::Water, vek::Rgb::new(0, 0, 0)),
                    }],
                },
                WiringAction {
                    formula: wiring::OutputFormula::Constant { value: 1.0 },
                    threshold: 1.0,
                    effects: vec![WiringActionEffect::SetLight {
                        r: wiring::OutputFormula::Input {
                            name: String::from("color"),
                        },
                        g: wiring::OutputFormula::Input {
                            name: String::from("color"),
                        },
                        b: wiring::OutputFormula::Input {
                            name: String::from("color"),
                        },
                    }],
                },
            ],
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        })
        .with(comp::Density(100_f32));
    let ent2 = builder2.build();

    pos.0.x += 3.0;
    let builder3 = server
        .state
        .create_wiring(pos, comp::object::Body::TrainingDummy, WiringElement {
            actions: vec![],
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        })
        .with(comp::Density(comp::object::Body::TrainingDummy.density().0))
        .with(Circuit {
            wires: vec![
                Wire {
                    input_entity: ent1,
                    input_field: String::from("deaths_last_tick"),
                    output_entity: ent1,
                    output_field: String::from("deaths_last_tick"),
                },
                Wire {
                    input_entity: ent1,
                    input_field: String::from("deaths_accumulated"),
                    output_entity: ent1,
                    output_field: String::from("deaths_accumulated"),
                },
                Wire {
                    input_entity: ent1,
                    input_field: String::from("pressed"),
                    output_entity: ent1,
                    output_field: String::from("pressed"),
                },
                Wire {
                    input_entity: ent1,
                    input_field: String::from("deaths_accumulated"),
                    output_entity: ent2,
                    output_field: String::from("deaths_accumulated"),
                },
            ],
        });
    builder3.build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, "Wire"),
    );
    Ok(())
}

fn handle_adminify(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(alias), desired_role) = parse_args!(args, String, String) {
        let desired_role = if let Some(mut desired_role) = desired_role {
            desired_role.make_ascii_lowercase();
            Some(match &*desired_role {
                "admin" => AdminRole::Admin,
                "moderator" => AdminRole::Moderator,
                _ => {
                    return Err(action.help_string());
                },
            })
        } else {
            None
        };
        let (player, player_uuid) = find_alias(server.state.ecs(), &alias)?;
        let client_uuid = uuid(server, client, "client")?;
        let uid = uid(server, player, "player")?;

        // Your permanent role, not your temporary role, is what's used to determine
        // what temporary roles you can grant.
        let client_real_role = real_role(server, client_uuid, "client")?;

        // This appears to prevent de-mod / de-admin for mods / admins with access to
        // this command, but it does not in the case where the target is
        // temporary, because `verify_above_role` always values permanent roles
        // above temporary ones.
        verify_above_role(
            server,
            (client, client_uuid),
            (player, player_uuid),
            "Cannot reassign a role for anyone with your role or higher.",
        )?;

        // Ensure that it's not possible to assign someone a higher role than your own
        // (i.e. even if mods had the ability to create temporary mods, they
        // wouldn't be able to create temporary admins).
        //
        // Also note that we perform no more permissions checks after this point based
        // on the assignee's temporary role--even if the player's temporary role
        // is higher than the client's, we still allow the role to be reduced to
        // the selected role, as long as they would have permission to assign it
        // in the first place.  This is consistent with our
        // policy on bans--banning or lengthening a ban (decreasing player permissions)
        // can be done even after an unban or ban shortening (increasing player
        // permissions) by someone with a higher role than the person doing the
        // ban.  So if we change how bans work, we should change how things work
        // here, too, for consistency.
        if desired_role > Some(client_real_role) {
            return Err(
                "Cannot assign someone a temporary role higher than your own permanent one".into(),
            );
        }

        let mut admin_storage = server.state.ecs().write_storage::<comp::Admin>();
        let entry = admin_storage
            .entry(player)
            .map_err(|_| "Cannot find player entity!".to_string())?;
        match (entry, desired_role) {
            (StorageEntry::Vacant(_), None) => {
                return Err("Player already has no role!".into());
            },
            (StorageEntry::Occupied(o), None) => {
                let old_role = o.remove().0;
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        format!("Role removed from player {}: {:?}", alias, old_role),
                    ),
                );
            },
            (entry, Some(desired_role)) => {
                let verb = match entry
                    .replace(comp::Admin(desired_role))
                    .map(|old_admin| old_admin.0.cmp(&desired_role))
                {
                    Some(Ordering::Equal) => {
                        return Err("Player already has that role!".into());
                    },
                    Some(Ordering::Greater) => "downgraded",
                    Some(Ordering::Less) | None => "upgraded",
                };
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        format!("Role for player {} {} to {:?}", alias, verb, desired_role),
                    ),
                );
            },
        };
        // Update player list so the player shows up as moderator in client chat.
        //
        // NOTE: We deliberately choose not to differentiate between moderators and
        // administrators in the player list.
        let is_moderator = desired_role.is_some();
        let msg = ServerGeneral::PlayerListUpdate(PlayerListUpdate::Moderator(uid, is_moderator));
        server.state.notify_players(msg);
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_tell(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;

    if let (Some(alias), message_opt) = parse_args!(args, String, ..Vec<String>) {
        let ecs = server.state.ecs();
        let player = find_alias(ecs, &alias)?.0;

        if player == target {
            return Err("You can't /tell yourself.".into());
        }
        let target_uid = uid(server, target, "target")?;
        let player_uid = uid(server, player, "player")?;
        let mode = comp::ChatMode::Tell(player_uid);
        insert_or_replace_component(server, target, mode.clone(), "target")?;
        let msg = if message_opt.is_empty() {
            format!("{} wants to talk to you.", alias)
        } else {
            message_opt.join(" ")
        };
        server.state.send_chat(mode.new_message(target_uid, msg));
        server.notify_client(target, ServerGeneral::ChatMode(mode));
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_faction(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;

    let factions = server.state.ecs().read_storage();
    if let Some(comp::Faction(faction)) = factions.get(target) {
        let mode = comp::ChatMode::Faction(faction.to_string());
        drop(factions);
        insert_or_replace_component(server, target, mode.clone(), "target")?;
        let msg = args.join(" ");
        if !msg.is_empty() {
            if let Some(uid) = server.state.ecs().read_storage().get(target) {
                server.state.send_chat(mode.new_message(*uid, msg));
            }
        }
        server.notify_client(target, ServerGeneral::ChatMode(mode));
        Ok(())
    } else {
        Err("Please join a faction with /join_faction".into())
    }
}

fn handle_group(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;

    let groups = server.state.ecs().read_storage::<comp::Group>();
    if let Some(group) = groups.get(target) {
        let mode = comp::ChatMode::Group(*group);
        drop(groups);
        insert_or_replace_component(server, target, mode.clone(), "target")?;
        let msg = args.join(" ");
        if !msg.is_empty() {
            if let Some(uid) = server.state.ecs().read_storage().get(target) {
                server.state.send_chat(mode.new_message(*uid, msg));
            }
        }
        server.notify_client(target, ServerGeneral::ChatMode(mode));
        Ok(())
    } else {
        Err("Please create a group first".into())
    }
}

fn handle_group_invite(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(target_alias) = parse_args!(args, String) {
        let target_player = find_alias(server.state.ecs(), &target_alias)?.0;
        let uid = uid(server, target_player, "player")?;

        server
            .state
            .mut_resource::<EventBus<ServerEvent>>()
            .emit_now(ServerEvent::InitiateInvite(target, uid, InviteKind::Group));

        if client != target {
            server.notify_client(
                target,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    format!("{} has been invited to your group.", target_alias),
                ),
            );
        }

        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                format!("Invited {} to the group.", target_alias),
            ),
        );
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_group_kick(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    // Checking if leader is already done in group_manip
    if let Some(target_alias) = parse_args!(args, String) {
        let target_player = find_alias(server.state.ecs(), &target_alias)?.0;
        let uid = uid(server, target_player, "player")?;

        server
            .state
            .mut_resource::<EventBus<ServerEvent>>()
            .emit_now(ServerEvent::GroupManip(target, comp::GroupManip::Kick(uid)));
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_group_leave(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    server
        .state
        .mut_resource::<EventBus<ServerEvent>>()
        .emit_now(ServerEvent::GroupManip(target, comp::GroupManip::Leave));
    Ok(())
}

fn handle_group_promote(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    // Checking if leader is already done in group_manip
    if let Some(target_alias) = parse_args!(args, String) {
        let target_player = find_alias(server.state.ecs(), &target_alias)?.0;
        let uid = uid(server, target_player, "player")?;

        server
            .state
            .mut_resource::<EventBus<ServerEvent>>()
            .emit_now(ServerEvent::GroupManip(
                target,
                comp::GroupManip::AssignLeader(uid),
            ));
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_region(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;

    let mode = comp::ChatMode::Region;
    insert_or_replace_component(server, target, mode.clone(), "target")?;
    let msg = args.join(" ");
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(target) {
            server.state.send_chat(mode.new_message(*uid, msg));
        }
    }
    server.notify_client(target, ServerGeneral::ChatMode(mode));
    Ok(())
}

fn handle_say(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;

    let mode = comp::ChatMode::Say;
    insert_or_replace_component(server, target, mode.clone(), "target")?;
    let msg = args.join(" ");
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(target) {
            server.state.send_chat(mode.new_message(*uid, msg));
        }
    }
    server.notify_client(target, ServerGeneral::ChatMode(mode));
    Ok(())
}

fn handle_world(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;

    let mode = comp::ChatMode::World;
    insert_or_replace_component(server, target, mode.clone(), "target")?;
    let msg = args.join(" ");
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(target) {
            server.state.send_chat(mode.new_message(*uid, msg));
        }
    }
    server.notify_client(target, ServerGeneral::ChatMode(mode));
    Ok(())
}

fn handle_join_faction(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let players = server.state.ecs().read_storage::<comp::Player>();
    if let Some(alias) = players.get(target).map(|player| player.alias.clone()) {
        drop(players);
        let (faction_leave, mode) = if let Some(faction) = parse_args!(args, String) {
            let mode = comp::ChatMode::Faction(faction.clone());
            insert_or_replace_component(server, target, mode.clone(), "target")?;
            let faction_join = server
                .state
                .ecs()
                .write_storage()
                .insert(target, comp::Faction(faction.clone()))
                .ok()
                .flatten()
                .map(|f| f.0);
            server.state.send_chat(
                ChatType::FactionMeta(faction.clone())
                    .chat_msg(format!("[{}] joined faction ({})", alias, faction)),
            );
            (faction_join, mode)
        } else {
            let mode = comp::ChatMode::default();
            insert_or_replace_component(server, target, mode.clone(), "target")?;
            let faction_leave = server
                .state
                .ecs()
                .write_storage()
                .remove(target)
                .map(|comp::Faction(f)| f);
            (faction_leave, mode)
        };
        if let Some(faction) = faction_leave {
            server.state.send_chat(
                ChatType::FactionMeta(faction.clone())
                    .chat_msg(format!("[{}] left faction ({})", alias, faction)),
            );
        }
        server.notify_client(target, ServerGeneral::ChatMode(mode));
        Ok(())
    } else {
        Err("Could not find your player alias".into())
    }
}

#[cfg(not(feature = "worldgen"))]
fn handle_debug_column(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    Err("Unsupported without worldgen enabled".into())
}

#[cfg(feature = "worldgen")]
fn handle_debug_column(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let sim = server.world.sim();
    let sampler = server.world.sample_columns();
    let wpos = if let (Some(x), Some(y)) = parse_args!(args, i32, i32) {
        Vec2::new(x, y)
    } else {
        let pos = position(server, target, "target")?;
        // FIXME: Deal with overflow, if needed.
        pos.0.xy().map(|x| x as i32)
    };
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
        Ok(())
    } else {
        Err("Not a pregenerated chunk.".into())
    }
}

fn handle_disconnect_all_players(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let client_uuid = uuid(server, client, "client")?;
    // Make sure temporary mods/admins can't run this command.
    let _role = real_role(server, client_uuid, "role")?;

    if parse_args!(args, String).as_deref() != Some("confirm") {
        return Err(
            "Please run the command again with the second argument of \"confirm\" to confirm that \
             you really want to disconnect all players from the server"
                .to_string(),
        );
    }

    let ecs = server.state.ecs();
    let players = &ecs.read_storage::<comp::Player>();

    // TODO: This logging and verification of admin commands would be better moved
    // to a more generic method used for auditing -all- admin commands.
    let player_name;
    if let Some(player) = players.get(client) {
        player_name = &*player.alias;
    } else {
        warn!(
            "Failed to get player name for admin who used /disconnect_all_players - ignoring \
             command."
        );
        return Err("You do not exist, so you cannot use this command".to_string());
    }

    info!(
        "Disconnecting all clients due to admin command from {}",
        player_name
    );
    server.disconnect_all_clients_requested = true;

    Ok(())
}

fn handle_skill_point(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(a_skill_tree), Some(sp), a_alias) = parse_args!(args, String, u16, String) {
        let skill_tree = parse_skill_tree(&a_skill_tree)?;
        let player = a_alias
            .map(|alias| find_alias(server.state.ecs(), &alias).map(|(target, _)| target))
            .unwrap_or(Ok(target))?;

        if let Some(mut skill_set) = server
            .state
            .ecs_mut()
            .write_storage::<comp::SkillSet>()
            .get_mut(player)
        {
            skill_set.add_skill_points(skill_tree, sp);
            Ok(())
        } else {
            Err("Player has no stats!".into())
        }
    } else {
        Err(action.help_string())
    }
}

fn parse_skill_tree(skill_tree: &str) -> CmdResult<comp::skills::SkillGroupKind> {
    use comp::{item::tool::ToolKind, skills::SkillGroupKind};
    match skill_tree {
        "general" => Ok(SkillGroupKind::General),
        "sword" => Ok(SkillGroupKind::Weapon(ToolKind::Sword)),
        "axe" => Ok(SkillGroupKind::Weapon(ToolKind::Axe)),
        "hammer" => Ok(SkillGroupKind::Weapon(ToolKind::Hammer)),
        "bow" => Ok(SkillGroupKind::Weapon(ToolKind::Bow)),
        "staff" => Ok(SkillGroupKind::Weapon(ToolKind::Staff)),
        "sceptre" => Ok(SkillGroupKind::Weapon(ToolKind::Sceptre)),
        "mining" => Ok(SkillGroupKind::Weapon(ToolKind::Pick)),
        _ => Err(format!("{} is not a skill group!", skill_tree)),
    }
}

fn handle_remove_lights(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    let opt_radius = parse_args!(args, f32);
    let player_pos = position(server, target, "target")?;
    let mut to_delete = vec![];

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

    let size = to_delete.len();

    for entity in to_delete {
        if let Err(e) = server.state.delete_entity_recorded(entity) {
            error!(?e, "Failed to delete light: {:?}", e);
        }
    }

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, format!("Removed {} lights!", size)),
    );
    Ok(())
}

fn handle_sudo(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(player_alias), Some(cmd), cmd_args) =
        parse_args!(args, String, String, ..Vec<String>)
    {
        if let Ok(action) = cmd.parse() {
            let (player, player_uuid) = find_alias(server.state.ecs(), &player_alias)?;
            let client_uuid = uuid(server, client, "client")?;
            verify_above_role(
                server,
                (client, client_uuid),
                (player, player_uuid),
                "Cannot sudo players with roles higher than your own.",
            )?;

            // TODO: consider making this into a tail call or loop (to avoid the potential
            // stack overflow, although it's less of a risk coming from only mods and
            // admins).
            do_command(server, client, player, cmd_args, &action)
        } else {
            Err(format!("Unknown command: /{}", cmd))
        }
    } else {
        Err(action.help_string())
    }
}

fn handle_version(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
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
    Ok(())
}

fn handle_whitelist(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    let now = Utc::now();

    if let (Some(whitelist_action), Some(username)) = parse_args!(args, String, String) {
        let client_uuid = uuid(server, client, "client")?;
        let client_username = uuid_to_username(server, client, client_uuid)?;
        let client_role = real_role(server, client_uuid, "client")?;

        if whitelist_action.eq_ignore_ascii_case("add") {
            let uuid = find_username(server, &username)?;

            let record = WhitelistRecord {
                date: now,
                info: Some(WhitelistInfo {
                    username_when_whitelisted: username.clone(),
                    whitelisted_by: client_uuid,
                    whitelisted_by_username: client_username,
                    whitelisted_by_role: client_role.into(),
                }),
            };

            let edit =
                server
                    .editable_settings_mut()
                    .whitelist
                    .edit(server.data_dir().as_ref(), |w| {
                        if w.insert(uuid, record).is_some() {
                            None
                        } else {
                            Some(format!("added to whitelist: {}", username))
                        }
                    });
            edit_setting_feedback(server, client, edit, || {
                format!("already in whitelist: {}!", username)
            })
        } else if whitelist_action.eq_ignore_ascii_case("remove") {
            let client_uuid = uuid(server, client, "client")?;
            let client_role = real_role(server, client_uuid, "client")?;

            let uuid = find_username(server, &username)?;
            let mut err_info = "not part of whitelist: ";
            let edit =
                server
                    .editable_settings_mut()
                    .whitelist
                    .edit(server.data_dir().as_ref(), |w| {
                        w.remove(&uuid)
                            .filter(|record| {
                                if record.whitelisted_by_role() <= client_role.into() {
                                    true
                                } else {
                                    err_info = "permission denied to remove user: ";
                                    false
                                }
                            })
                            .map(|_| format!("removed from whitelist: {}", username))
                    });
            edit_setting_feedback(server, client, edit, || format!("{}{}", err_info, username))
        } else {
            Err(action.help_string())
        }
    } else {
        Err(action.help_string())
    }
}

fn kick_player(
    server: &mut Server,
    (client, client_uuid): (EcsEntity, Uuid),
    (target_player, target_player_uuid): (EcsEntity, Uuid),
    reason: &str,
) -> CmdResult<()> {
    verify_above_role(
        server,
        (client, client_uuid),
        (target_player, target_player_uuid),
        "Cannot kick players with roles higher than your own.",
    )?;
    server.notify_client(
        target_player,
        ServerGeneral::Disconnect(DisconnectReason::Kicked(reason.to_string())),
    );
    server
        .state
        .mut_resource::<EventBus<ServerEvent>>()
        .emit_now(ServerEvent::ClientDisconnect(
            target_player,
            common::comp::DisconnectReason::Kicked,
        ));
    Ok(())
}

fn handle_kick(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(target_alias), reason_opt) = parse_args!(args, String, String) {
        let client_uuid = uuid(server, client, "client")?;
        let reason = reason_opt.unwrap_or_default();
        let ecs = server.state.ecs();
        let target_player = find_alias(ecs, &target_alias)?;

        kick_player(server, (client, client_uuid), target_player, &reason)?;
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
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_ban(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(username), overwrite, parse_duration, reason_opt) =
        parse_args!(args, String, bool, HumanDuration, String)
    {
        let reason = reason_opt.unwrap_or_default();
        let overwrite = overwrite.unwrap_or(false);

        let player_uuid = find_username(server, &username)?;

        let client_uuid = uuid(server, client, "client")?;
        let client_username = uuid_to_username(server, client, client_uuid)?;
        let client_role = real_role(server, client_uuid, "client")?;

        let now = Utc::now();
        let end_date = parse_duration
            .map(|duration| chrono::Duration::from_std(duration.into()))
            .transpose()
            .map_err(|err| format!("Error converting to duration: {}", err))?
            // On overflow (someone adding some ridiculous timespan), just make the ban infinite.
            .and_then(|duration| now.checked_add_signed(duration));

        let ban_info = BanInfo {
            performed_by: client_uuid,
            performed_by_username: client_username,
            performed_by_role: client_role.into(),
        };

        let ban = Ban {
            reason: reason.clone(),
            info: Some(ban_info),
            end_date,
        };

        let edit = server
            .editable_settings_mut()
            .banlist
            .ban_action(
                server.data_dir().as_ref(),
                now,
                player_uuid,
                username.clone(),
                BanAction::Ban(ban),
                overwrite,
            )
            .map(|result| {
                (
                    format!("Added {} to the banlist with reason: {}", username, reason),
                    result,
                )
            });

        edit_setting_feedback(server, client, edit, || {
            format!("{} is already on the banlist", username)
        })?;
        // If the player is online kick them (this may fail if the player is a hardcoded
        // admin; we don't care about that case because hardcoded admins can log on even
        // if they're on the ban list).
        let ecs = server.state.ecs();
        if let Ok(target_player) = find_uuid(ecs, player_uuid) {
            let _ = kick_player(
                server,
                (client, client_uuid),
                (target_player, player_uuid),
                &reason,
            );
        }
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_battlemode(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ChatCommand,
) -> CmdResult<()> {
    // TODO: discuss time
    const COOLDOWN: f64 = 60.0 * 5.0;

    let ecs = server.state.ecs();
    let time = ecs.read_resource::<Time>();
    let settings = ecs.read_resource::<Settings>();
    if let Some(mode) = parse_args!(args, String) {
        if !settings.battle_mode.allow_choosing() {
            return Err("Command disabled in server settings".to_owned());
        }

        #[cfg(feature = "worldgen")]
        let in_town = {
            // get chunk position
            let pos = position(server, target, "target")?;
            let wpos = pos.0.xy().map(|x| x as i32);
            let chunk_pos = wpos.map2(TerrainChunkSize::RECT_SIZE, |wpos, size: u32| {
                wpos / size as i32
            });
            server.world.civs().sites().any(|site| {
                // empirical
                const RADIUS: f32 = 9.0;
                let delta = site
                    .center
                    .map(|x| x as f32)
                    .distance(chunk_pos.map(|x| x as f32));
                delta < RADIUS
            })
        };
        // just skip this check, if worldgen is disabled
        #[cfg(not(feature = "worldgen"))]
        let in_town = true;

        if !in_town {
            return Err("You need to be in town to change battle mode!".to_owned());
        }

        let mut players = ecs.write_storage::<comp::Player>();
        let mut player_info = players.get_mut(target).ok_or_else(|| {
            error!("Can't get player component for player");
            "Error!"
        })?;
        if let Some(Time(last_change)) = player_info.last_battlemode_change {
            let Time(time) = *time;
            let elapsed = time - last_change;
            if elapsed < COOLDOWN {
                let msg = format!(
                    "Cooldown period active. Try again in {:.0} seconds",
                    COOLDOWN - elapsed,
                );
                return Err(msg);
            }
        }
        let mode = match mode.as_str() {
            "pvp" => BattleMode::PvP,
            "pve" => BattleMode::PvE,
            _ => return Err("Available modes: pvp, pve".to_owned()),
        };
        if player_info.battle_mode == mode {
            return Err("Attempted to set the same battlemode".to_owned());
        }
        player_info.battle_mode = mode;
        player_info.last_battlemode_change = Some(*time);
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                format!("New battle mode: {:?}", mode),
            ),
        );
        Ok(())
    } else {
        let players = ecs.read_storage::<comp::Player>();
        let player = players.get(target).ok_or_else(|| {
            error!("Can't get player component for player");
            "Error!"
        })?;
        let mut msg = format!("Current battle mode: {:?}.", player.battle_mode);
        if settings.battle_mode.allow_choosing() {
            msg.push_str(" Possible to change.");
        } else {
            msg.push_str(" Global.");
        }
        if let Some(change) = player.last_battlemode_change {
            let Time(time) = *time;
            let Time(change) = change;
            let elapsed = time - change;
            let next = COOLDOWN - elapsed;
            let notice = format!(" Next change will be available in: {:.0} seconds", next);
            msg.push_str(&notice);
        }
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, msg),
        );
        Ok(())
    }
}

fn handle_battlemode_force(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    let ecs = server.state.ecs();
    let settings = ecs.read_resource::<Settings>();
    if !settings.battle_mode.allow_choosing() {
        return Err("Command disabled in server settings".to_owned());
    }
    let mode = parse_args!(args, String).ok_or_else(|| action.help_string())?;
    let mode = match mode.as_str() {
        "pvp" => BattleMode::PvP,
        "pve" => BattleMode::PvE,
        _ => return Err("Available modes: pvp, pve".to_owned()),
    };
    let mut players = ecs.write_storage::<comp::Player>();
    let mut player_info = players
        .get_mut(target)
        .ok_or("Cannot get player component for target")?;
    player_info.battle_mode = mode;
    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            format!("Set battle mode to: {:?}", mode),
        ),
    );
    Ok(())
}

fn handle_unban(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(username) = parse_args!(args, String) {
        let player_uuid = find_username(server, &username)?;

        let client_uuid = uuid(server, client, "client")?;
        let client_username = uuid_to_username(server, client, client_uuid)?;
        let client_role = real_role(server, client_uuid, "client")?;

        let now = Utc::now();

        let ban_info = BanInfo {
            performed_by: client_uuid,
            performed_by_username: client_username,
            performed_by_role: client_role.into(),
        };

        let unban = BanAction::Unban(ban_info);

        let edit = server
            .editable_settings_mut()
            .banlist
            .ban_action(
                server.data_dir().as_ref(),
                now,
                player_uuid,
                username.clone(),
                unban,
                false,
            )
            .map(|result| (format!("{} was successfully unbanned", username), result));

        edit_setting_feedback(server, client, edit, || {
            format!("{} was already unbanned", username)
        })
    } else {
        Err(action.help_string())
    }
}

fn handle_server_physics(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(username), enabled_opt) = parse_args!(args, String, bool) {
        let uuid = find_username(server, &username)?;
        let server_force = enabled_opt.unwrap_or(true);

        let mut player_physics_settings =
            server.state.ecs().write_resource::<PlayerPhysicsSettings>();
        let entry = player_physics_settings.settings.entry(uuid).or_default();
        entry.server_force = server_force;

        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                format!(
                    "Updated physics settings for {} ({}): {:?}",
                    username, uuid, entry
                ),
            ),
        );
        Ok(())
    } else {
        Err(action.help_string())
    }
}

fn handle_apply_buff(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let (Some(buff), strength, duration) = parse_args!(args, String, f32, f64) {
        let strength = strength.unwrap_or(0.01);
        let duration = Duration::from_secs_f64(duration.unwrap_or(1.0));
        let buffdata = BuffData::new(strength, Some(duration));
        if buff != "all" {
            cast_buff(&buff, buffdata, server, target)
        } else {
            for kind in BUFF_PACK.iter() {
                cast_buff(kind, buffdata, server, target)?;
            }
            Ok(())
        }
    } else {
        Err(action.help_string())
    }
}

fn cast_buff(kind: &str, data: BuffData, server: &mut Server, target: EcsEntity) -> CmdResult<()> {
    if let Some(buffkind) = parse_buffkind(kind) {
        let ecs = &server.state.ecs();
        let mut buffs_all = ecs.write_storage::<comp::Buffs>();
        if let Some(mut buffs) = buffs_all.get_mut(target) {
            buffs.insert(Buff::new(buffkind, data, vec![], BuffSource::Command));
        }
        Ok(())
    } else {
        Err(format!("unknown buff: {}", kind))
    }
}

fn parse_buffkind(buff: &str) -> Option<BuffKind> { BUFF_PARSER.get(buff).copied() }

fn handle_skill_preset(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ChatCommand,
) -> CmdResult<()> {
    if let Some(preset) = parse_args!(args, String) {
        if let Some(mut skill_set) = server
            .state
            .ecs_mut()
            .write_storage::<comp::SkillSet>()
            .get_mut(target)
        {
            match preset.as_str() {
                "clear" => {
                    clear_skillset(&mut skill_set);
                    Ok(())
                },
                preset => set_skills(&mut skill_set, preset),
            }
        } else {
            Err("Player has no stats!".into())
        }
    } else {
        Err(action.help_string())
    }
}

fn clear_skillset(skill_set: &mut comp::SkillSet) { *skill_set = comp::SkillSet::default(); }

fn set_skills(skill_set: &mut comp::SkillSet, preset: &str) -> CmdResult<()> {
    let presets = match common::cmd::SkillPresetManifest::load(PRESET_MANIFEST_PATH) {
        Ok(presets) => presets.read().0.clone(),
        Err(err) => {
            warn!("Error in preset: {}", err);
            return Err("Error while loading presets".to_owned());
        },
    };
    if let Some(preset) = presets.get(preset) {
        for (skill, level) in preset {
            let group = if let Some(group) = skill.skill_group_kind() {
                group
            } else {
                warn!("Skill in preset doesn't exist in any group");
                return Err("Preset is broken".to_owned());
            };
            for _ in 0..*level {
                let cost = skill_set.skill_cost(*skill);
                skill_set.add_skill_points(group, cost);
                skill_set.unlock_skill(*skill);
            }
        }
        Ok(())
    } else {
        Err("Such preset doesn't exist".to_owned())
    }
}
