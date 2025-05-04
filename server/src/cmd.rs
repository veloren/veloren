//! # Implementing new commands.
//! To implement a new command provide a handler function
//! in [do_command].
#[cfg(feature = "worldgen")]
use crate::weather::WeatherJob;
use crate::{
    Server, Settings, StateExt,
    client::Client,
    location::Locations,
    login_provider::LoginProvider,
    settings::{
        BanInfo, BanOperation, BanOperationError, EditableSetting, SettingError, WhitelistInfo,
        WhitelistRecord, banlist::NormalizedIpAddr, server_description::ServerDescription,
        server_physics::ServerPhysicsForceRecord,
    },
    sys::terrain::SpawnEntityData,
    wiring::{self, OutputFormula},
};
#[cfg(feature = "worldgen")]
use common::{cmd::SPOT_PARSER, spot::Spot};

use assets::AssetExt;
use authc::Uuid;
use chrono::{NaiveTime, Timelike, Utc};
use common::{
    CachedSpatialGrid, Damage, DamageKind, DamageSource, Explosion, GroupTarget, LoadoutBuilder,
    RadiusEffect, assets,
    calendar::Calendar,
    cmd::{
        AreaKind, BUFF_PACK, BUFF_PARSER, EntityTarget, KIT_MANIFEST_PATH, KitSpec,
        PRESET_MANIFEST_PATH, ServerChatCommand,
    },
    combat,
    comp::{
        self, AdminRole, Aura, AuraKind, BuffCategory, ChatType, Content, GizmoSubscriber,
        Inventory, Item, LightEmitter, LocalizationArg, WaypointArea,
        agent::{FlightMode, PidControllers},
        aura::{AuraKindVariant, AuraTarget},
        buff::{Buff, BuffData, BuffKind, BuffSource, DestInfo, MiscBuffData},
        inventory::{
            item::{MaterialStatManifest, Quality, all_items_expect, tool::AbilityMap},
            slot::Slot,
        },
        invite::InviteKind,
        misc::PortalData,
    },
    depot,
    effect::Effect,
    event::{
        ClientDisconnectEvent, CreateNpcEvent, CreateSpecialEntityEvent, EventBus, ExplosionEvent,
        GroupManipEvent, InitiateInviteEvent, PermanentChange, TamePetEvent,
    },
    generation::{EntityConfig, EntityInfo, SpecialEntity},
    link::Is,
    mounting::{Rider, Volume, VolumeRider},
    npc::{self, get_npc_name},
    outcome::Outcome,
    parse_cmd_args,
    resources::{BattleMode, ProgramTime, Secs, Time, TimeOfDay, TimeScale},
    rtsim::{Actor, Role},
    spiral::Spiral2d,
    terrain::{Block, BlockKind, CoordinateConversions, SpriteKind, StructureSprite},
    tether::Tethered,
    uid::Uid,
    vol::ReadVol,
};
#[cfg(feature = "worldgen")]
use common::{
    terrain::{TERRAIN_CHUNK_BLOCKS_LG, TerrainChunkSize},
    weather,
};
use common_net::{
    msg::{DisconnectReason, Notification, PlayerListUpdate, ServerGeneral},
    sync::WorldSyncExt,
};
use common_state::{Areas, AreasContainer, BuildArea, NoDurabilityArea, SpecialAreaError, State};
use core::{cmp::Ordering, convert::TryFrom};
use hashbrown::{HashMap, HashSet};
use humantime::Duration as HumanDuration;
use rand::{Rng, thread_rng};
use specs::{Builder, Entity as EcsEntity, Join, LendJoin, WorldExt, storage::StorageEntry};
use std::{
    fmt::Write, net::SocketAddr, num::NonZeroU32, ops::DerefMut, str::FromStr, sync::Arc,
    time::Duration,
};
use vek::*;
use wiring::{Circuit, Wire, WireNode, WiringAction, WiringActionEffect, WiringElement};
#[cfg(feature = "worldgen")]
use world::util::{LOCALITY, Sampler};

use common::comp::Alignment;
use tracing::{error, info, warn};

pub trait ChatCommandExt {
    fn execute(&self, server: &mut Server, entity: EcsEntity, args: Vec<String>);
}
impl ChatCommandExt for ServerChatCommand {
    fn execute(&self, server: &mut Server, entity: EcsEntity, args: Vec<String>) {
        if let Err(err) = do_command(server, entity, entity, args, self) {
            server.notify_client(
                entity,
                ServerGeneral::server_msg(ChatType::CommandError, err),
            );
        }
    }
}

type CmdResult<T> = Result<T, Content>;

/// Handler function called when the command is executed.
/// # Arguments
/// * `&mut Server` - the `Server` instance executing the command.
/// * `EcsEntity` - an `Entity` corresponding to the player that invoked the
///   command.
/// * `EcsEntity` - an `Entity` for the player on whom the command is invoked.
///   This differs from the previous argument when using /sudo
/// * `Vec<String>` - a `Vec<String>` containing the arguments of the command
///   after the keyword.
/// * `&ChatCommand` - the command to execute with the above arguments --
///   Handler functions must parse arguments from the the given `String`
///   (`parse_args!` exists for this purpose).
///
/// # Returns
///
/// A `Result` that is `Ok` if the command went smoothly, and `Err` if it
/// failed; on failure, the string is sent to the client who initiated the
/// command.
type CommandHandler =
    fn(&mut Server, EcsEntity, EcsEntity, Vec<String>, &ServerChatCommand) -> CmdResult<()>;

fn do_command(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    cmd: &ServerChatCommand,
) -> CmdResult<()> {
    // Make sure your role is at least high enough to execute this command.
    if cmd.needs_role() > server.entity_admin_role(client) {
        return Err(Content::localized_with_args("command-no-permission", [(
            "command_name",
            cmd.keyword(),
        )]));
    }

    let handler: CommandHandler = match cmd {
        ServerChatCommand::Adminify => handle_adminify,
        ServerChatCommand::Airship => handle_spawn_airship,
        ServerChatCommand::Alias => handle_alias,
        ServerChatCommand::AreaAdd => handle_area_add,
        ServerChatCommand::AreaList => handle_area_list,
        ServerChatCommand::AreaRemove => handle_area_remove,
        ServerChatCommand::Aura => handle_aura,
        ServerChatCommand::Ban => handle_ban,
        ServerChatCommand::BanIp => handle_ban_ip,
        ServerChatCommand::BattleMode => handle_battlemode,
        ServerChatCommand::BattleModeForce => handle_battlemode_force,
        ServerChatCommand::Body => handle_body,
        ServerChatCommand::Buff => handle_buff,
        ServerChatCommand::Build => handle_build,
        ServerChatCommand::Campfire => handle_spawn_campfire,
        ServerChatCommand::ClearPersistedTerrain => handle_clear_persisted_terrain,
        ServerChatCommand::DeathEffect => handle_death_effect,
        ServerChatCommand::DebugColumn => handle_debug_column,
        ServerChatCommand::DebugWays => handle_debug_ways,
        ServerChatCommand::DisconnectAllPlayers => handle_disconnect_all_players,
        ServerChatCommand::DropAll => handle_drop_all,
        ServerChatCommand::Dummy => handle_spawn_training_dummy,
        ServerChatCommand::Explosion => handle_explosion,
        ServerChatCommand::Faction => handle_faction,
        ServerChatCommand::GiveItem => handle_give_item,
        ServerChatCommand::Gizmos => handle_gizmos,
        ServerChatCommand::GizmosRange => handle_gizmos_range,
        ServerChatCommand::Goto => handle_goto,
        ServerChatCommand::GotoRand => handle_goto_rand,
        ServerChatCommand::Group => handle_group,
        ServerChatCommand::GroupInvite => handle_group_invite,
        ServerChatCommand::GroupKick => handle_group_kick,
        ServerChatCommand::GroupLeave => handle_group_leave,
        ServerChatCommand::GroupPromote => handle_group_promote,
        ServerChatCommand::Health => handle_health,
        ServerChatCommand::IntoNpc => handle_into_npc,
        ServerChatCommand::JoinFaction => handle_join_faction,
        ServerChatCommand::Jump => handle_jump,
        ServerChatCommand::Kick => handle_kick,
        ServerChatCommand::Kill => handle_kill,
        ServerChatCommand::KillNpcs => handle_kill_npcs,
        ServerChatCommand::Kit => handle_kit,
        ServerChatCommand::Lantern => handle_lantern,
        ServerChatCommand::Light => handle_light,
        ServerChatCommand::MakeBlock => handle_make_block,
        ServerChatCommand::MakeNpc => handle_make_npc,
        ServerChatCommand::MakeSprite => handle_make_sprite,
        ServerChatCommand::Motd => handle_motd,
        ServerChatCommand::Object => handle_object,
        ServerChatCommand::Outcome => handle_outcome,
        ServerChatCommand::PermitBuild => handle_permit_build,
        ServerChatCommand::Players => handle_players,
        ServerChatCommand::Portal => handle_spawn_portal,
        ServerChatCommand::ResetRecipes => handle_reset_recipes,
        ServerChatCommand::Region => handle_region,
        ServerChatCommand::ReloadChunks => handle_reload_chunks,
        ServerChatCommand::RemoveLights => handle_remove_lights,
        ServerChatCommand::Respawn => handle_respawn,
        ServerChatCommand::RevokeBuild => handle_revoke_build,
        ServerChatCommand::RevokeBuildAll => handle_revoke_build_all,
        ServerChatCommand::Safezone => handle_safezone,
        ServerChatCommand::Say => handle_say,
        ServerChatCommand::ServerPhysics => handle_server_physics,
        ServerChatCommand::SetBodyType => handle_set_body_type,
        ServerChatCommand::SetMotd => handle_set_motd,
        ServerChatCommand::SetWaypoint => handle_set_waypoint,
        ServerChatCommand::Ship => handle_spawn_ship,
        ServerChatCommand::Site => handle_site,
        ServerChatCommand::SkillPoint => handle_skill_point,
        ServerChatCommand::SkillPreset => handle_skill_preset,
        ServerChatCommand::Spawn => handle_spawn,
        ServerChatCommand::Spot => handle_spot,
        ServerChatCommand::Sudo => handle_sudo,
        ServerChatCommand::Tell => handle_tell,
        ServerChatCommand::Time => handle_time,
        ServerChatCommand::TimeScale => handle_time_scale,
        ServerChatCommand::Tp => handle_tp,
        ServerChatCommand::RtsimTp => handle_rtsim_tp,
        ServerChatCommand::RtsimInfo => handle_rtsim_info,
        ServerChatCommand::RtsimNpc => handle_rtsim_npc,
        ServerChatCommand::RtsimPurge => handle_rtsim_purge,
        ServerChatCommand::RtsimChunk => handle_rtsim_chunk,
        ServerChatCommand::Unban => handle_unban,
        ServerChatCommand::UnbanIp => handle_unban_ip,
        ServerChatCommand::Version => handle_version,
        ServerChatCommand::Wiring => handle_spawn_wiring,
        ServerChatCommand::Whitelist => handle_whitelist,
        ServerChatCommand::World => handle_world,
        ServerChatCommand::MakeVolume => handle_make_volume,
        ServerChatCommand::Location => handle_location,
        ServerChatCommand::CreateLocation => handle_create_location,
        ServerChatCommand::DeleteLocation => handle_delete_location,
        ServerChatCommand::WeatherZone => handle_weather_zone,
        ServerChatCommand::Lightning => handle_lightning,
        ServerChatCommand::Scale => handle_scale,
        ServerChatCommand::RepairEquipment => handle_repair_equipment,
        ServerChatCommand::Tether => handle_tether,
        ServerChatCommand::DestroyTethers => handle_destroy_tethers,
        ServerChatCommand::Mount => handle_mount,
        ServerChatCommand::Dismount => handle_dismount,
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
        .ok_or_else(|| {
            Content::localized_with_args("command-position-unavailable", [("target", descriptor)])
        })
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
        .map_err(|_| Content::localized_with_args("command-entity-dead", [("entity", descriptor)]))
}

fn uuid(server: &Server, entity: EcsEntity, descriptor: &str) -> CmdResult<Uuid> {
    server
        .state
        .ecs()
        .read_storage::<comp::Player>()
        .get(entity)
        .map(|player| player.uuid())
        .ok_or_else(|| {
            Content::localized_with_args("command-player-info-unavailable", [(
                "target", descriptor,
            )])
        })
}

fn socket_addr(server: &Server, entity: EcsEntity, descriptor: &str) -> CmdResult<SocketAddr> {
    server
        .state
        .ecs()
        .read_storage::<Client>()
        .get(entity)
        .ok_or_else(|| {
            Content::localized_with_args("command-entity-has-no-client", [("target", descriptor)])
        })?
        .connected_from_addr()
        .socket_addr()
        .ok_or_else(|| {
            Content::localized_with_args("command-client-has-no-socketaddr", [(
                "target", descriptor,
            )])
        })
}

fn real_role(server: &Server, uuid: Uuid, descriptor: &str) -> CmdResult<AdminRole> {
    server
        .editable_settings()
        .admins
        .get(&uuid)
        .map(|record| record.role.into())
        .ok_or_else(|| {
            Content::localized_with_args("command-player-role-unavailable", [(
                "target", descriptor,
            )])
        })
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
        .ok_or_else(|| {
            Content::localized_with_args("command-uid-unavailable", [("target", descriptor)])
        })
}

fn area(server: &mut Server, area_name: &str, kind: &str) -> CmdResult<depot::Id<Aabb<i32>>> {
    get_areas_mut(kind, &mut server.state)?
        .area_metas()
        .get(area_name)
        .copied()
        .ok_or_else(|| {
            Content::localized_with_args("command-area-not-found", [("area", area_name)])
        })
}

// Prevent use through sudo.
fn no_sudo(client: EcsEntity, target: EcsEntity) -> CmdResult<()> {
    if client == target {
        Ok(())
    } else {
        // This happens when [ab]using /sudo
        Err(Content::localized("command-no-sudo"))
    }
}

fn can_send_message(target: EcsEntity, server: &mut Server) -> CmdResult<()> {
    if server
        .state
        .ecs()
        .read_storage::<Client>()
        .get(target)
        .is_none_or(|client| !client.client_type.can_send_message())
    {
        Err(Content::localized("command-cannot-send-message-hidden"))
    } else {
        Ok(())
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
    reason: Content,
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
        Err(reason)
    }
}

fn find_alias(ecs: &specs::World, alias: &str, find_hidden: bool) -> CmdResult<(EcsEntity, Uuid)> {
    (
        &ecs.entities(),
        &ecs.read_storage::<comp::Player>(),
        &ecs.read_storage::<Client>(),
    )
        .join()
        .find(|(_, player, client)| {
            // If `find_hidden` is set to false, disallow discovering this player using ie.
            // /tell or /group_invite
            player.alias == alias && (client.client_type.emit_login_events() || find_hidden)
        })
        .map(|(entity, player, _)| (entity, player.uuid()))
        .ok_or_else(|| {
            Content::localized_with_args("command-player-not-found", [("player", alias)])
        })
}

fn find_uuid(ecs: &specs::World, uuid: Uuid) -> CmdResult<EcsEntity> {
    (&ecs.entities(), &ecs.read_storage::<comp::Player>())
        .join()
        .find(|(_, player)| player.uuid() == uuid)
        .map(|(entity, _)| entity)
        .ok_or_else(|| {
            Content::localized_with_args("command-player-uuid-not-found", [(
                "uuid",
                uuid.to_string(),
            )])
        })
}

fn find_username(server: &mut Server, username: &str) -> CmdResult<Uuid> {
    server
        .state
        .mut_resource::<LoginProvider>()
        .username_to_uuid(username)
        .map_err(|_| {
            Content::localized_with_args("command-username-uuid-unavailable", [(
                "username", username,
            )])
        })
}

/// NOTE: Intended to be run only on logged-in clients.
fn uuid_to_username(
    server: &mut Server,
    fallback_entity: EcsEntity,
    uuid: Uuid,
) -> CmdResult<String> {
    let make_err = || {
        Content::localized_with_args("command-uuid-username-unavailable", [(
            "uuid",
            uuid.to_string(),
        )])
    };
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
    result: Option<(Content, Result<(), SettingError<S>>)>,
    failure: impl FnOnce() -> Content,
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
        Err(setting_error) => edit_setting_error_feedback(server, client, setting_error, || info),
    }
}

fn edit_banlist_feedback(
    server: &mut Server,
    client: EcsEntity,
    result: Result<(), BanOperationError>,
    // Message to provide if the edit was succesful (even if an IO error occurred, since the
    // setting will still be changed in memory)
    info: impl FnOnce() -> Content,
    // Message to provide if the edit was cancelled due to it having no effect.
    failure: impl FnOnce() -> Content,
) -> CmdResult<()> {
    match result {
        Ok(()) => {
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandInfo, info()),
            );
            Ok(())
        },
        // TODO: whether there is a typo and the supplied username has no ban entry or if the
        // target was already banned/unbanned, the user of this command will always get the same
        // error message here, which seems like it could be misleading.
        Err(BanOperationError::NoEffect) => Err(failure()),
        Err(BanOperationError::EditFailed(setting_error)) => {
            edit_setting_error_feedback(server, client, setting_error, info)
        },
    }
}

fn edit_setting_error_feedback<S: EditableSetting>(
    server: &mut Server,
    client: EcsEntity,
    setting_error: SettingError<S>,
    info: impl FnOnce() -> Content,
) -> CmdResult<()> {
    match setting_error {
        SettingError::Io(err) => {
            let info = info();
            warn!(
                ?err,
                "Failed to write settings file to disk, but succeeded in memory (success message: \
                 {:?})",
                info,
            );
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandError,
                    Content::localized_with_args("command-error-write-settings", [
                        ("error", Content::Plain(format!("{:?}", err))),
                        ("message", info),
                    ]),
                ),
            );
            Ok(())
        },
        SettingError::Integrity(err) => Err(Content::localized_with_args(
            "command-error-while-evaluating-request",
            [("error", format!("{err:?}"))],
        )),
    }
}

fn handle_drop_all(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;

    let mut items = Vec::new();
    if let Some(mut inventory) = server
        .state
        .ecs()
        .write_storage::<Inventory>()
        .get_mut(target)
    {
        items = inventory.drain().collect();
    }

    let mut rng = thread_rng();

    let item_to_place = items
        .into_iter()
        .filter(|i| !matches!(i.quality(), Quality::Debug));
    for item in item_to_place {
        let vel = Vec3::new(rng.gen_range(-0.1..0.1), rng.gen_range(-0.1..0.1), 0.5);

        server.state.create_item_drop(
            comp::Pos(Vec3::new(
                pos.0.x + rng.gen_range(5.0..10.0),
                pos.0.y + rng.gen_range(5.0..10.0),
                pos.0.z + 5.0,
            )),
            comp::Ori::default(),
            comp::Vel(vel),
            comp::PickupItem::new(item, ProgramTime(server.state.get_program_time()), true),
            None,
        );
    }

    Ok(())
}

fn handle_give_item(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(item_name), give_amount_opt) = parse_cmd_args!(args, String, u32) {
        let give_amount = give_amount_opt.unwrap_or(1);
        if let Ok(item) = Item::new_from_asset(&item_name.replace(['/', '\\'], "."))
            .inspect_err(|error| error!(?error, "Failed to parse item asset!"))
        {
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
                    .write_storage::<Inventory>()
                    .get_mut(target)
                    .map(|mut inv| {
                        // NOTE: Deliberately ignores items that couldn't be pushed.
                        if inv.push(item).is_err() {
                            res = Err(Content::localized_with_args(
                                "command-give-inventory-full",
                                [("total", give_amount as u64), ("given", 0)],
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
                    .write_storage::<Inventory>()
                    .get_mut(target)
                    .map(|mut inv| {
                        for i in 0..give_amount {
                            // NOTE: Deliberately ignores items that couldn't be pushed.
                            if inv.push(item.duplicate(&ability_map, &msm)).is_err() {
                                res = Err(Content::localized_with_args(
                                    "command-give-inventory-full",
                                    [("total", give_amount as u64), ("given", i as u64)],
                                ));
                                break;
                            }
                        }
                    });
            }

            if res.is_ok() {
                let msg = ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::localized_with_args("command-give-inventory-success", [
                        ("total", LocalizationArg::from(give_amount as u64)),
                        ("item", LocalizationArg::from(item_name)),
                    ]),
                );
                server.notify_client(client, msg);
            }

            let mut inventory_update = server
                .state
                .ecs_mut()
                .write_storage::<comp::InventoryUpdate>();
            if let Some(update) = inventory_update.get_mut(target) {
                update.push(comp::InventoryUpdateEvent::Given);
            } else {
                inventory_update
                    .insert(
                        target,
                        comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Given),
                    )
                    .map_err(|_| Content::Plain("Entity target is dead!".to_string()))?;
            }
            res
        } else {
            Err(Content::localized_with_args("command-invalid-item", [(
                "item", item_name,
            )]))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_gizmos(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(kind), gizmo_target) = parse_cmd_args!(args, String, EntityTarget) {
        let mut subscribers = server.state().ecs().write_storage::<GizmoSubscriber>();

        let gizmo_target = gizmo_target
            .map(|gizmo_target| get_entity_target(gizmo_target, server))
            .transpose()?
            .map(|gizmo_target| {
                server
                    .state()
                    .ecs()
                    .read_storage()
                    .get(gizmo_target)
                    .copied()
                    .ok_or(Content::localized("command-entity-dead"))
            })
            .transpose()?;

        match kind.as_str() {
            "All" => {
                let subscriber = subscribers
                    .entry(target)
                    .map_err(|_| Content::localized("command-entity-dead"))?
                    .or_insert_with(Default::default);
                let context = match gizmo_target {
                    Some(uid) => comp::gizmos::GizmoContext::EnabledWithTarget(uid),
                    None => comp::gizmos::GizmoContext::Enabled,
                };
                for (_, ctx) in subscriber.gizmos.iter_mut() {
                    *ctx = context.clone();
                }
                Ok(())
            },
            "None" => {
                subscribers.remove(target);
                Ok(())
            },
            s => {
                if let Ok(kind) = comp::gizmos::GizmoSubscription::from_str(s) {
                    let subscriber = subscribers
                        .entry(target)
                        .map_err(|_| Content::localized("command-entity-dead"))?
                        .or_insert_with(Default::default);

                    subscriber.gizmos[kind] = match gizmo_target {
                        Some(uid) => comp::gizmos::GizmoContext::EnabledWithTarget(uid),
                        None => match subscriber.gizmos[kind] {
                            comp::gizmos::GizmoContext::Disabled => {
                                comp::gizmos::GizmoContext::Enabled
                            },
                            comp::gizmos::GizmoContext::Enabled
                            | comp::gizmos::GizmoContext::EnabledWithTarget(_) => {
                                comp::gizmos::GizmoContext::Disabled
                            },
                        },
                    };

                    Ok(())
                } else {
                    Err(action.help_content())
                }
            },
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_gizmos_range(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(range) = parse_cmd_args!(args, f32) {
        let mut subscribers = server.state().ecs().write_storage::<GizmoSubscriber>();
        subscribers
            .entry(target)
            .map_err(|_| Content::localized("command-entity-dead"))?
            .or_insert_with(Default::default)
            .range = range;

        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_make_block(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(block_name), r, g, b) = parse_cmd_args!(args, String, u8, u8, u8) {
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
            Err(Content::localized_with_args(
                "command-invalid-block-kind",
                [("kind", block_name)],
            ))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_into_npc(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    use crate::events::shared::{TransformEntityError, transform_entity};

    if client != target {
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized("command-into_npc-warning"),
            ),
        );
    }

    let Some(entity_config) = parse_cmd_args!(args, String) else {
        return Err(action.help_content());
    };

    let config = match EntityConfig::load(&entity_config) {
        Ok(asset) => asset.read(),
        Err(_err) => {
            return Err(Content::localized_with_args(
                "command-entity-load-failed",
                [("config", entity_config)],
            ));
        },
    };

    let mut loadout_rng = thread_rng();
    let entity_info = EntityInfo::at(
        server
            .state
            .read_component_copied::<comp::Pos>(target)
            .map(|p| p.0)
            .unwrap_or_default(),
    )
    .with_entity_config(config.clone(), Some(&entity_config), &mut loadout_rng, None);

    transform_entity(server, target, entity_info, true).map_err(|error| match error {
        TransformEntityError::EntityDead => {
            Content::localized_with_args("command-entity-dead", [("entity", "target")])
        },
        TransformEntityError::UnexpectedSpecialEntity => {
            Content::localized("command-unimplemented-spawn-special")
        },
        TransformEntityError::LoadingCharacter => {
            Content::localized("command-transform-invalid-presence")
        },
        TransformEntityError::EntityIsPlayer => {
            unreachable!(
                "Transforming players must be valid as we explicitly allowed player transformation"
            );
        },
    })
}

fn handle_make_npc(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let (entity_config, number) = parse_cmd_args!(args, String, i8);

    let entity_config = entity_config.ok_or_else(|| action.help_content())?;
    let number = match number {
        // Number of entities must be larger than 1
        Some(i8::MIN..=0) => {
            return Err(Content::localized("command-nof-entities-at-least"));
        },
        // But lower than 50
        Some(50..=i8::MAX) => {
            return Err(Content::localized("command-nof-entities-less-than"));
        },
        Some(number) => number,
        None => 1,
    };

    let config = match EntityConfig::load(&entity_config) {
        Ok(asset) => asset.read(),
        Err(_err) => {
            return Err(Content::localized_with_args(
                "command-entity-load-failed",
                [("config", entity_config)],
            ));
        },
    };

    let mut loadout_rng = thread_rng();
    for _ in 0..number {
        let comp::Pos(pos) = position(server, target, "target")?;
        let entity_info = EntityInfo::at(pos).with_entity_config(
            config.clone(),
            Some(&entity_config),
            &mut loadout_rng,
            None,
        );

        match SpawnEntityData::from_entity_info(entity_info) {
            SpawnEntityData::Special(_, _) => {
                return Err(Content::localized("command-unimplemented-spawn-special"));
            },
            SpawnEntityData::Npc(data) => {
                let (npc_builder, _pos) = data.to_npc_builder();

                server
                    .state
                    .ecs()
                    .read_resource::<EventBus<CreateNpcEvent>>()
                    .emit_now(CreateNpcEvent {
                        pos: comp::Pos(pos),
                        ori: comp::Ori::default(),
                        npc: npc_builder,
                    });
            },
        };
    }

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized_with_args("command-spawned-entities-config", [
                ("n", number.to_string()),
                ("config", entity_config),
            ]),
        ),
    );

    Ok(())
}

fn handle_make_sprite(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(sprite_name) = parse_cmd_args!(args, String) {
        let pos = position(server, target, "target")?;
        let pos = pos.0.map(|e| e.floor() as i32);
        let old_block = server
                .state
                .get_block(pos)
                // TODO: Make more principled.
                .unwrap_or_else(|| Block::air(SpriteKind::Empty));
        let set_block = |block| {
            server.state.set_block(pos, block);
            #[cfg(feature = "persistent_world")]
            if let Some(terrain_persistence) = server
                .state
                .ecs()
                .try_fetch_mut::<crate::TerrainPersistence>()
                .as_mut()
            {
                terrain_persistence.set_block(pos, block);
            }
        };
        if let Ok(sk) = SpriteKind::try_from(sprite_name.as_str()) {
            set_block(old_block.with_sprite(sk));

            Ok(())
        } else if let Ok(sprite) = ron::from_str::<StructureSprite>(sprite_name.as_str()) {
            set_block(sprite.get_block(|s| old_block.with_sprite(s)));

            Ok(())
        } else {
            Err(Content::localized_with_args("command-invalid-sprite", [(
                "kind",
                sprite_name,
            )]))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_motd(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let locale = server
        .state
        .ecs()
        .read_storage::<Client>()
        .get(client)
        .and_then(|client| client.locale.clone());

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::Plain(
                server
                    .editable_settings()
                    .server_description
                    .get(locale.as_deref())
                    .map_or("", |d| &d.motd)
                    .to_string(),
            ),
        ),
    );
    Ok(())
}

fn handle_set_motd(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let data_dir = server.data_dir();
    let client_uuid = uuid(server, client, "client")?;
    // Ensure the person setting this has a real role in the settings file, since
    // it's persistent.
    let _client_real_role = real_role(server, client_uuid, "client")?;
    match parse_cmd_args!(args, String, String) {
        (Some(locale), Some(msg)) => {
            let edit =
                server
                    .editable_settings_mut()
                    .server_description
                    .edit(data_dir.as_ref(), |d| {
                        let info = Content::localized_with_args(
                            "command-set_motd-message-added",
                            [("message", format!("{:?}", msg))],
                        );

                        if let Some(description) = d.descriptions.get_mut(&locale) {
                            description.motd = msg;
                        } else {
                            d.descriptions.insert(locale, ServerDescription {
                                motd: msg,
                                rules: None,
                            });
                        }

                        Some(info)
                    });
            drop(data_dir);
            edit_setting_feedback(server, client, edit, || {
                unreachable!("edit always returns Some")
            })
        },
        (Some(locale), None) => {
            let edit =
                server
                    .editable_settings_mut()
                    .server_description
                    .edit(data_dir.as_ref(), |d| {
                        if let Some(description) = d.descriptions.get_mut(&locale) {
                            description.motd.clear();
                            Some(Content::localized("command-set_motd-message-removed"))
                        } else {
                            Some(Content::localized("command-set_motd-message-not-set"))
                        }
                    });
            drop(data_dir);
            edit_setting_feedback(server, client, edit, || {
                unreachable!("edit always returns Some")
            })
        },
        _ => Err(action.help_content()),
    }
}

fn handle_set_body_type(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(new_body_type), permanent) = parse_cmd_args!(args, String, bool) {
        let permananet = permanent.unwrap_or(false);
        let body = server
            .state
            .ecs()
            .read_storage::<comp::Body>()
            .get(target)
            .copied();
        if let Some(mut body) = body {
            fn parse_body_type<B: FromStr + std::fmt::Display>(
                input: &str,
                all_types: impl IntoIterator<Item = B>,
            ) -> CmdResult<B> {
                FromStr::from_str(input).map_err(|_| {
                    Content::localized_with_args("cmd-set_body_type-not_found", [(
                        "options",
                        all_types
                            .into_iter()
                            .map(|b| b.to_string())
                            .reduce(|mut a, b| {
                                a.push_str(",\n");
                                a.push_str(&b);
                                a
                            })
                            .unwrap_or_default(),
                    )])
                })
            }
            let old_body = body;
            match &mut body {
                comp::Body::Humanoid(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::humanoid::ALL_BODY_TYPES)?;
                },
                comp::Body::QuadrupedSmall(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::quadruped_small::ALL_BODY_TYPES)?;
                },
                comp::Body::QuadrupedMedium(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::quadruped_medium::ALL_BODY_TYPES)?;
                },
                comp::Body::BirdMedium(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::bird_medium::ALL_BODY_TYPES)?;
                },
                comp::Body::FishMedium(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::fish_medium::ALL_BODY_TYPES)?;
                },
                comp::Body::Dragon(body) => {
                    body.body_type = parse_body_type(&new_body_type, comp::dragon::ALL_BODY_TYPES)?;
                },
                comp::Body::BirdLarge(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::bird_large::ALL_BODY_TYPES)?;
                },
                comp::Body::FishSmall(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::fish_small::ALL_BODY_TYPES)?;
                },
                comp::Body::BipedLarge(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::biped_large::ALL_BODY_TYPES)?;
                },
                comp::Body::BipedSmall(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::biped_small::ALL_BODY_TYPES)?;
                },
                comp::Body::Object(_) => {},
                comp::Body::Golem(body) => {
                    body.body_type = parse_body_type(&new_body_type, comp::golem::ALL_BODY_TYPES)?;
                },
                comp::Body::Theropod(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::theropod::ALL_BODY_TYPES)?;
                },
                comp::Body::QuadrupedLow(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::quadruped_low::ALL_BODY_TYPES)?;
                },
                comp::Body::Ship(_) => {},
                comp::Body::Arthropod(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::arthropod::ALL_BODY_TYPES)?;
                },
                comp::Body::Item(_) => {},
                comp::Body::Crustacean(body) => {
                    body.body_type =
                        parse_body_type(&new_body_type, comp::crustacean::ALL_BODY_TYPES)?;
                },
                comp::Body::Plugin(_) => {},
            };

            if old_body != body {
                assign_body(server, target, body)?;

                if permananet {
                    if let (
                        Some(new_body),
                        Some(player),
                        Some(comp::Presence {
                            kind: comp::PresenceKind::Character(id),
                            ..
                        }),
                    ) = (
                        server.state.ecs().read_storage::<comp::Body>().get(target),
                        server
                            .state
                            .ecs()
                            .read_storage::<comp::Player>()
                            .get(target),
                        server
                            .state
                            .ecs()
                            .read_storage::<comp::Presence>()
                            .get(target),
                    ) {
                        server
                            .state()
                            .ecs()
                            .write_resource::<crate::persistence::character_updater::CharacterUpdater>()
                            .edit_character(
                                target,
                                player.uuid().to_string(),
                                *id,
                                None,
                                (*new_body,),
                                Some(PermanentChange {
                                    expected_old_body: old_body,
                                }),
                            );
                    } else {
                        return Err(Content::localized("cmd-set_body_type-not_character"));
                    }
                }
            }
            Ok(())
        } else {
            Err(Content::localized("cmd-set_body_type-no_body"))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_jump(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(x), Some(y), Some(z), dismount_volume) = parse_cmd_args!(args, f32, f32, f32, bool)
    {
        server
            .state
            .position_mut(target, dismount_volume.unwrap_or(true), |current_pos| {
                current_pos.0 += Vec3::new(x, y, z)
            })
    } else {
        Err(action.help_content())
    }
}

fn handle_goto(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(x), Some(y), Some(z), dismount_volume) = parse_cmd_args!(args, f32, f32, f32, bool)
    {
        server
            .state
            .position_mut(target, dismount_volume.unwrap_or(true), |current_pos| {
                current_pos.0 = Vec3::new(x, y, z)
            })
    } else {
        Err(action.help_content())
    }
}

#[cfg(feature = "worldgen")]
fn handle_goto_rand(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let mut rng = rand::thread_rng();
    let map_size = server.world.sim().map_size_lg().vec();
    let chunk_side = 2_u32.pow(TERRAIN_CHUNK_BLOCKS_LG);
    let pos2d = Vec2::new(
        rng.gen_range(0..(2_u32.pow(map_size.x) * chunk_side)) as f32,
        rng.gen_range(0..(2_u32.pow(map_size.y) * chunk_side)) as f32,
    );
    let pos3d = pos2d.with_z(server.world.sim().get_surface_alt_approx(pos2d.as_()));
    server.state.position_mut(
        target,
        parse_cmd_args!(args, bool).unwrap_or(true),
        |current_pos| current_pos.0 = pos3d,
    )
}

#[cfg(not(feature = "worldgen"))]
fn handle_goto_rand(
    _server: &mut Server,
    _client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    Err(Content::Plain(
        "Unsupported without worldgen enabled".into(),
    ))
}

#[cfg(not(feature = "worldgen"))]
fn handle_site(
    _server: &mut Server,
    _client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    Err(Content::Plain(
        "Unsupported without worldgen enabled".into(),
    ))
}

/// TODO: Add autocompletion if possible (might require modifying enum to handle
/// dynamic values).
#[cfg(feature = "worldgen")]
fn handle_site(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(dest_name), dismount_volume) = parse_cmd_args!(args, String, bool) {
        let site = server
            .world
            .civs()
            .sites()
            .find(|site| {
                site.site_tmp
                    .is_some_and(|id| server.index.sites[id].name() == dest_name)
            })
            .ok_or_else(|| Content::localized("command-site-not-found"))?;

        let site_pos = server.world.find_accessible_pos(
            server.index.as_index_ref(),
            TerrainChunkSize::center_wpos(site.center),
            false,
        );

        server
            .state
            .position_mut(target, dismount_volume.unwrap_or(true), |current_pos| {
                current_pos.0 = site_pos
            })
    } else {
        Err(action.help_content())
    }
}

fn handle_respawn(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let waypoint = server
        .state
        .read_storage::<comp::Waypoint>()
        .get(target)
        .ok_or(Content::localized("command-respawn-no-waypoint"))?
        .get_pos();

    server.state.position_mut(target, true, |current_pos| {
        current_pos.0 = waypoint;
    })
}

fn handle_kill(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
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
    _action: &ServerChatCommand,
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

    let time = parse_cmd_args!(args, String);
    const EMSG: &str = "time always valid";
    let new_time = match time.as_deref() {
        Some("midnight") => next_cycle(
            NaiveTime::from_hms_opt(0, 0, 0)
                .expect(EMSG)
                .num_seconds_from_midnight() as f64,
        ),
        Some("night") => next_cycle(
            NaiveTime::from_hms_opt(20, 0, 0)
                .expect(EMSG)
                .num_seconds_from_midnight() as f64,
        ),
        Some("dawn") => next_cycle(
            NaiveTime::from_hms_opt(5, 0, 0)
                .expect(EMSG)
                .num_seconds_from_midnight() as f64,
        ),
        Some("morning") => next_cycle(
            NaiveTime::from_hms_opt(8, 0, 0)
                .expect(EMSG)
                .num_seconds_from_midnight() as f64,
        ),
        Some("day") => next_cycle(
            NaiveTime::from_hms_opt(10, 0, 0)
                .expect(EMSG)
                .num_seconds_from_midnight() as f64,
        ),
        Some("noon") => next_cycle(
            NaiveTime::from_hms_opt(12, 0, 0)
                .expect(EMSG)
                .num_seconds_from_midnight() as f64,
        ),
        Some("dusk") => next_cycle(
            NaiveTime::from_hms_opt(17, 0, 0)
                .expect(EMSG)
                .num_seconds_from_midnight() as f64,
        ),
        Some(n) => match n.parse::<f64>() {
            Ok(n) => {
                // Incase the number of digits in the number is greater than 16
                if n >= 1e17 {
                    return Err(Content::localized_with_args(
                        "command-time-parse-too-large",
                        [("n", n.to_string())],
                    ));
                }
                if n < 0.0 {
                    return Err(Content::localized_with_args(
                        "command-time-parse-negative",
                        [("n", n.to_string())],
                    ));
                }
                // Seconds from next midnight
                next_cycle(0.0) + n
            },
            Err(_) => match NaiveTime::parse_from_str(n, "%H:%M") {
                // Relative to current day
                Ok(time) => next_cycle(time.num_seconds_from_midnight() as f64),
                // Accept `u12345`, seconds since midnight day 0
                Err(_) => match n
                    .get(1..)
                    .filter(|_| n.starts_with('u'))
                    .and_then(|n| n.trim_start_matches('u').parse::<u64>().ok())
                {
                    // Absolute time (i.e. from world epoch)
                    Some(n) => {
                        if (n as f64) < time_in_seconds {
                            return Err(Content::localized_with_args("command-time-backwards", [
                                ("t", n),
                            ]));
                        }
                        n as f64
                    },
                    None => {
                        return Err(Content::localized_with_args("command-time-invalid", [(
                            "t", n,
                        )]));
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
            // this Herculean task? This code is gibberish! The last of the core Rust
            // dev team died exactly 337,194 years ago! Rust is now a long-forgotten
            // dialect of the ancient ones, lost to the sands of time. Ashes to ashes,
            // dust to dust. When all hope is lost, one particularly intrepid
            // post-human hominid exployed by the 'Veloren Revival Corp' (no doubt we
            // still won't have gotten rid of this blasted 'capitalism' thing by then)
            // might notice, after years of searching, a particularly curious
            // inscription within the code. The letters `D`, `A`, `Y`. Curious! She
            // consults the post-human hominid scholars of the old. Care to empathise
            // with her shock when she discovers that these symbols, as alien as they
            // may seem, correspond exactly to the word `ⓕя𝐢ᵇᵇ𝔩Ｅ`, the word for
            // 'day' in the post-human hominid language, which is of course universal.
            // Imagine also her surprise when, after much further translating, she
            // finds a comment predicting her very existence and her struggle to
            // decode this great mystery. Rejoice! The Veloren Revival Corp. may now
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
                Some(time) => Content::localized_with_args("command-time-current", [(
                    "t",
                    time.format("%H:%M").to_string(),
                )]),
                None => Content::localized("command-time-unknown"),
            };
            server.notify_client(
                client,
                ServerGeneral::server_msg(ChatType::CommandInfo, msg),
            );
            return Ok(());
        },
    };

    server.state.mut_resource::<TimeOfDay>().0 = new_time;
    let time = server.state.ecs().read_resource::<Time>();

    // Update all clients with the new TimeOfDay (without this they would have to
    // wait for the next 100th tick to receive the update).
    let mut tod_lazymsg = None;
    let clients = server.state.ecs().read_storage::<Client>();
    let calendar = server.state.ecs().read_resource::<Calendar>();
    let time_scale = server.state.ecs().read_resource::<TimeScale>();
    for client in (&clients).join() {
        let msg = tod_lazymsg.unwrap_or_else(|| {
            client.prepare(ServerGeneral::TimeOfDay(
                TimeOfDay(new_time),
                (*calendar).clone(),
                *time,
                *time_scale,
            ))
        });
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
                Content::Plain(format!("Time changed to: {}", new_time.format("%H:%M"))),
            ),
        );
    }
    Ok(())
}

fn handle_time_scale(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let time_scale = server
        .state
        .ecs_mut()
        .get_mut::<TimeScale>()
        .expect("Expected time scale to be added.");
    if args.is_empty() {
        let time_scale = time_scale.0;
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized_with_args("command-time_scale-current", [(
                    "scale",
                    time_scale.to_string(),
                )]),
            ),
        );
    } else if let Some(scale) = parse_cmd_args!(args, f64) {
        time_scale.0 = scale.clamp(0.0001, 1000.0);
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized_with_args("command-time_scale-changed", [(
                    "scale",
                    scale.to_string(),
                )]),
            ),
        );
        // Update all clients with the new TimeOfDay (without this they would have to
        // wait for the next 100th tick to receive the update).
        let mut tod_lazymsg = None;
        let clients = server.state.ecs().read_storage::<Client>();
        let time = server.state.ecs().read_resource::<Time>();
        let time_of_day = server.state.ecs().read_resource::<TimeOfDay>();
        let calendar = server.state.ecs().read_resource::<Calendar>();
        for client in (&clients).join() {
            let msg = tod_lazymsg.unwrap_or_else(|| {
                client.prepare(ServerGeneral::TimeOfDay(
                    *time_of_day,
                    (*calendar).clone(),
                    *time,
                    TimeScale(scale),
                ))
            });
            let _ = client.send_prepared(&msg);
            tod_lazymsg = Some(msg);
        }
    } else {
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandError,
                Content::Plain("Wrong parameter, expected f32.".to_string()),
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
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(hp) = parse_cmd_args!(args, f32) {
        if let Some(mut health) = server
            .state
            .ecs()
            .write_storage::<comp::Health>()
            .get_mut(target)
        {
            let time = server.state.ecs().read_resource::<Time>();
            let change = comp::HealthChange {
                amount: hp - health.current(),
                by: None,
                cause: None,
                precise: false,
                time: *time,
                instance: rand::random(),
            };
            health.change_by(change);
            Ok(())
        } else {
            Err(Content::Plain("You have no health".into()))
        }
    } else {
        Err(Content::Plain("You must specify health amount!".into()))
    }
}

fn handle_alias(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(alias) = parse_cmd_args!(args, String) {
        // Prevent silly aliases
        comp::Player::alias_validate(&alias).map_err(|e| Content::Plain(e.to_string()))?;

        let old_alias_optional = server
            .state
            .ecs_mut()
            .write_storage::<comp::Player>()
            .get_mut(target)
            .map(|mut player| std::mem::replace(&mut player.alias, alias));

        // Update name on client player lists
        let ecs = server.state.ecs();
        if let (Some(uid), Some(player), Some(client), Some(old_alias)) = (
            ecs.read_storage::<Uid>().get(target),
            ecs.read_storage::<comp::Player>().get(target),
            ecs.read_storage::<Client>().get(target),
            old_alias_optional,
        ) && client.client_type.emit_login_events()
        {
            let msg = ServerGeneral::PlayerListUpdate(PlayerListUpdate::Alias(
                *uid,
                player.alias.clone(),
            ));
            server.state.notify_players(msg);

            // Announce alias change if target has a Body.
            if ecs.read_storage::<comp::Body>().get(target).is_some() {
                server.state.notify_players(ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::Plain(format!("{} is now known as {}.", old_alias, player.alias)),
                ));
            }
        }
        if client != target {
            // Notify target that an admin changed the alias due to /sudo
            server.notify_client(
                target,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::Plain("An admin changed your alias.".to_string()),
                ),
            );
        }
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_tp(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let (entity_target, dismount_volume) = parse_cmd_args!(args, EntityTarget, bool);
    let player = if let Some(entity_target) = entity_target {
        get_entity_target(entity_target, server)?
    } else if client != target {
        client
    } else {
        return Err(action.help_content());
    };
    let player_pos = position(server, player, "player")?;
    server
        .state
        .position_mut(target, dismount_volume.unwrap_or(true), |target_pos| {
            *target_pos = player_pos
        })
}

fn handle_rtsim_tp(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    use crate::rtsim::RtSim;
    let (npc_id, dismount_volume) = parse_cmd_args!(args, u64, bool);
    let pos = if let Some(id) = npc_id {
        server
            .state
            .ecs()
            .read_resource::<RtSim>()
            .state()
            .data()
            .npcs
            .values()
            .find(|npc| npc.uid == id)
            .ok_or_else(|| Content::Plain(format!("No NPC has the id {id}")))?
            .wpos
    } else {
        return Err(action.help_content());
    };
    server
        .state
        .position_mut(target, dismount_volume.unwrap_or(true), |target_pos| {
            target_pos.0 = pos;
        })
}

fn handle_rtsim_info(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    use crate::rtsim::RtSim;
    if let Some(id) = parse_cmd_args!(args, u64) {
        let rtsim = server.state.ecs().read_resource::<RtSim>();
        let data = rtsim.state().data();
        let (id, npc) = data
            .npcs
            .iter()
            .find(|(_, npc)| npc.uid == id)
            .ok_or_else(|| Content::Plain(format!("No NPC has the id {id}")))?;

        let mut info = String::new();

        let _ = writeln!(&mut info, "-- General Information --");
        let _ = writeln!(&mut info, "Seed: {}", npc.seed);
        let _ = writeln!(&mut info, "Pos: {:?}", npc.wpos);
        let _ = writeln!(&mut info, "Role: {:?}", npc.role);
        let _ = writeln!(&mut info, "Home: {:?}", npc.home);
        let _ = writeln!(&mut info, "Faction: {:?}", npc.faction);
        let _ = writeln!(&mut info, "Personality: {:?}", npc.personality);
        let _ = writeln!(&mut info, "-- Status --");
        let _ = writeln!(&mut info, "Current site: {:?}", npc.current_site);
        let _ = writeln!(&mut info, "Current mode: {:?}", npc.mode);
        let _ = writeln!(
            &mut info,
            "Riding: {:?}",
            data.npcs
                .mounts
                .get_mount_link(id)
                .map(|link| data.npcs.get(link.mount).map_or(0, |mount| mount.uid))
        );
        let _ = writeln!(&mut info, "-- Action State --");
        if let Some(brain) = &npc.brain {
            let mut bt = Vec::new();
            brain.action.backtrace(&mut bt);
            for (i, action) in bt.into_iter().enumerate() {
                let _ = writeln!(&mut info, "[{}] {}", i, action);
            }
        } else {
            let _ = writeln!(&mut info, "<NPC has no brain>");
        }

        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, Content::Plain(info)),
        );

        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_rtsim_npc(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    use crate::rtsim::RtSim;
    if let (Some(query), count) = parse_cmd_args!(args, String, u32) {
        let terms = query
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_lowercase())
            .collect::<Vec<_>>();
        let npc_names = &*common::npc::NPC_NAMES.read();
        let rtsim = server.state.ecs().read_resource::<RtSim>();
        let data = rtsim.state().data();
        let mut npcs = data
            .npcs
            .values()
            .filter(|npc| {
                let mut tags = vec![
                    npc.profession()
                        .map(|p| format!("{:?}", p))
                        .unwrap_or_default(),
                    match &npc.role {
                        Role::Civilised(_) => "civilised".to_string(),
                        Role::Wild => "wild".to_string(),
                        Role::Monster => "monster".to_string(),
                        Role::Vehicle => "vehicle".to_string(),
                    },
                    format!("{:?}", npc.mode),
                    format!("{}", npc.uid),
                    npc_names[&npc.body].keyword.clone(),
                ];
                if let Some(species_meta) = npc_names.get_species_meta(&npc.body) {
                    tags.push(species_meta.keyword.clone());
                }
                if let Some(brain) = &npc.brain {
                    rtsim::ai::Action::backtrace(&brain.action, &mut tags);
                }
                terms.iter().all(|term| {
                    tags.iter()
                        .any(|tag| tag.trim().to_lowercase().contains(term.as_str()))
                })
            })
            .collect::<Vec<_>>();
        if let Ok(pos) = position(server, target, "target") {
            npcs.sort_by_key(|npc| (npc.wpos.distance_squared(pos.0) * 10.0) as u64);
        }

        let mut info = String::new();

        let _ = writeln!(&mut info, "-- NPCs matching [{}] --", terms.join(", "));
        for npc in npcs.iter().take(count.unwrap_or(!0) as usize) {
            let _ = write!(&mut info, "{} ({}), ", npc.get_name(), npc.uid);
        }
        let _ = writeln!(&mut info);
        let _ = writeln!(
            &mut info,
            "Showing {}/{} matching NPCs.",
            count.unwrap_or(npcs.len() as u32),
            npcs.len()
        );

        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, Content::Plain(info)),
        );

        Ok(())
    } else {
        Err(action.help_content())
    }
}

// TODO: Remove this command when rtsim becomes more mature and we're sure we
// don't need purges to fix broken state.
fn handle_rtsim_purge(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    use crate::rtsim::RtSim;
    let client_uuid = uuid(server, client, "client")?;
    if !matches!(real_role(server, client_uuid, "client")?, AdminRole::Admin) {
        return Err(Content::localized("command-rtsim-purge-perms"));
    }

    if let Some(should_purge) = parse_cmd_args!(args, bool) {
        server
            .state
            .ecs()
            .write_resource::<RtSim>()
            .set_should_purge(should_purge);
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::Plain(format!(
                    "Rtsim data {} be purged on next startup",
                    if should_purge { "WILL" } else { "will NOT" },
                )),
            ),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_rtsim_chunk(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    use crate::rtsim::{ChunkStates, RtSim};
    let pos = position(server, target, "target")?;

    let chunk_key = pos.0.xy().as_::<i32>().wpos_to_cpos();

    let rtsim = server.state.ecs().read_resource::<RtSim>();
    let data = rtsim.state().data();

    let chunk_states = rtsim.state().resource::<ChunkStates>();
    let chunk_state = match chunk_states.0.get(chunk_key) {
        Some(Some(chunk_state)) => chunk_state,
        Some(None) => {
            return Err(Content::localized_with_args("command-chunk-not-loaded", [
                ("x", chunk_key.x.to_string()),
                ("y", chunk_key.y.to_string()),
            ]));
        },
        None => {
            return Err(Content::localized_with_args(
                "command-chunk-out-of-bounds",
                [
                    ("x", chunk_key.x.to_string()),
                    ("y", chunk_key.y.to_string()),
                ],
            ));
        },
    };

    let mut info = String::new();
    let _ = writeln!(
        &mut info,
        "-- Chunk {}, {} Resources --",
        chunk_key.x, chunk_key.y
    );
    for (res, frac) in data.nature.get_chunk_resources(chunk_key) {
        let total = chunk_state.max_res[res];
        let _ = writeln!(
            &mut info,
            "{:?}: {} / {} ({}%)",
            res,
            frac * total as f32,
            total,
            frac * 100.0
        );
    }

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, Content::Plain(info)),
    );

    Ok(())
}

fn handle_spawn(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    match parse_cmd_args!(args, String, npc::NpcBody, u32, bool, f32, bool) {
        (
            Some(opt_align),
            Some(npc::NpcBody(id, mut body)),
            opt_amount,
            opt_ai,
            opt_scale,
            opt_tethered,
        ) => {
            let uid = uid(server, target, "target")?;
            let alignment = parse_alignment(uid, &opt_align)?;

            if matches!(alignment, Alignment::Owned(_))
                && server
                    .state
                    .ecs()
                    .read_storage::<comp::Anchor>()
                    .contains(target)
            {
                return Err(Content::Plain(
                    "Spawning this pet would create an anchor chain".into(),
                ));
            }

            let amount = opt_amount.filter(|x| *x > 0).unwrap_or(1).min(50);

            let ai = opt_ai.unwrap_or(true);
            let pos = position(server, target, "target")?;
            let mut agent = comp::Agent::from_body(&body());

            if matches!(alignment, comp::Alignment::Owned(_)) {
                agent.psyche.idle_wander_factor = 0.25;
            } else {
                // If unowned, the agent should stay in a particular place
                agent = agent.with_patrol_origin(pos.0);
            }

            for _ in 0..amount {
                let vel = Vec3::new(
                    thread_rng().gen_range(-2.0..3.0),
                    thread_rng().gen_range(-2.0..3.0),
                    10.0,
                );

                let body = body();
                let loadout = LoadoutBuilder::from_default(&body).build();
                let inventory = Inventory::with_loadout(loadout, body);

                let mut entity_base = server
                    .state
                    .create_npc(
                        pos,
                        comp::Ori::default(),
                        comp::Stats::new(
                            Content::Plain(get_npc_name(id, npc::BodyType::from_body(body))),
                            body,
                        ),
                        comp::SkillSet::default(),
                        Some(comp::Health::new(body)),
                        comp::Poise::new(body),
                        inventory,
                        body,
                        opt_scale.map(comp::Scale).unwrap_or(body.scale()),
                    )
                    .with(comp::Vel(vel))
                    .with(alignment);

                if ai {
                    entity_base = entity_base.with(agent.clone());
                }

                let new_entity = entity_base.build();

                if opt_tethered == Some(true) {
                    let tether_leader = server
                        .state
                        .read_component_cloned::<Is<Rider>>(target)
                        .map(|is_rider| is_rider.mount)
                        .or_else(|| server.state.ecs().uid_from_entity(target));
                    let tether_follower = server.state.ecs().uid_from_entity(new_entity);

                    if let (Some(leader), Some(follower)) = (tether_leader, tether_follower) {
                        server
                            .state
                            .link(Tethered {
                                leader,
                                follower,
                                tether_length: 4.0,
                            })
                            .map_err(|_| Content::Plain("Failed to tether entities".to_string()))?;
                    } else {
                        return Err(Content::Plain("Tether members don't have Uids.".into()));
                    }
                }

                // Add to group system if a pet
                if matches!(alignment, comp::Alignment::Owned { .. }) {
                    server.state.emit_event_now(TamePetEvent {
                        owner_entity: target,
                        pet_entity: new_entity,
                    });
                } else if let Some(group) = alignment.group() {
                    insert_or_replace_component(server, new_entity, group, "new entity")?;
                }

                if let Some(uid) = server.state.ecs().uid_from_entity(new_entity) {
                    server.notify_client(
                        client,
                        ServerGeneral::server_msg(
                            ChatType::CommandInfo,
                            Content::localized_with_args("command-spawned-entity", [("id", uid.0)]),
                        ),
                    );
                }
            }
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::Plain(format!("Spawned {} entities", amount)),
                ),
            );
            Ok(())
        },
        _ => Err(action.help_content()),
    }
}

fn handle_spawn_training_dummy(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;
    let vel = Vec3::new(
        thread_rng().gen_range(-2.0..3.0),
        thread_rng().gen_range(-2.0..3.0),
        10.0,
    );

    let body = comp::Body::Object(comp::object::Body::TrainingDummy);

    let stats = comp::Stats::new(
        Content::with_attr("name-custom-village-dummy", "neut"),
        body,
    );
    let skill_set = comp::SkillSet::default();
    let health = comp::Health::new(body);
    let poise = comp::Poise::new(body);

    server
        .state
        .create_npc(
            pos,
            comp::Ori::default(),
            stats,
            skill_set,
            Some(health),
            poise,
            Inventory::with_empty(),
            body,
            comp::Scale(1.0),
        )
        .with(comp::Vel(vel))
        .build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized("command-spawned-dummy"),
        ),
    );
    Ok(())
}

fn handle_spawn_airship(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let (body_name, angle) = parse_cmd_args!(args, String, f32);
    let mut pos = position(server, target, "target")?;
    pos.0.z += 50.0;
    const DESTINATION_RADIUS: f32 = 2000.0;
    let angle = angle.map(|a| a * std::f32::consts::PI / 180.0);
    let dir = angle.map(|a| Vec3::new(a.cos(), a.sin(), 0.0));
    let destination = dir.map(|dir| pos.0 + dir * DESTINATION_RADIUS + Vec3::new(0.0, 0.0, 200.0));
    let ship = if let Some(body_name) = body_name {
        *comp::ship::ALL_AIRSHIPS
            .iter()
            .find(|body| format!("{body:?}") == body_name)
            .ok_or_else(|| Content::Plain(format!("No such airship '{body_name}'.")))?
    } else {
        comp::ship::Body::random_airship_with(&mut thread_rng())
    };
    let ori = comp::Ori::from(common::util::Dir::new(dir.unwrap_or(Vec3::unit_y())));
    let mut builder = server
        .state
        .create_ship(pos, ori, ship, |ship| ship.make_collider());
    if let Some(pos) = destination {
        let agent = comp::Agent::from_body(&comp::Body::Ship(ship))
            .with_destination(pos)
            .with_altitude_pid_controller(PidControllers::<16>::new_multi_pid_controllers(
                FlightMode::FlyThrough,
                pos,
            ));
        builder = builder.with(agent);
    }
    builder.build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized("command-spawned-airship"),
        ),
    );
    Ok(())
}

fn handle_spawn_ship(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let (body_name, tethered, angle) = parse_cmd_args!(args, String, bool, f32);
    let mut pos = position(server, target, "target")?;
    pos.0.z += 2.0;
    const DESTINATION_RADIUS: f32 = 2000.0;
    let angle = angle.map(|a| a * std::f32::consts::PI / 180.0);
    let dir = angle.map(|a| Vec3::new(a.cos(), a.sin(), 0.0));
    let destination = dir.map(|dir| pos.0 + dir * DESTINATION_RADIUS + Vec3::new(0.0, 0.0, 200.0));
    let ship = if let Some(body_name) = body_name {
        *comp::ship::ALL_SHIPS
            .iter()
            .find(|body| format!("{body:?}") == body_name)
            .ok_or_else(|| Content::Plain(format!("No such airship '{body_name}'.")))?
    } else {
        comp::ship::Body::random_airship_with(&mut thread_rng())
    };
    let ori = comp::Ori::from(common::util::Dir::new(dir.unwrap_or(Vec3::unit_y())));
    let mut builder = server
        .state
        .create_ship(pos, ori, ship, |ship| ship.make_collider());

    if let Some(pos) = destination {
        let agent = comp::Agent::from_body(&comp::Body::Ship(ship))
            .with_destination(pos)
            .with_altitude_pid_controller(PidControllers::<16>::new_multi_pid_controllers(
                FlightMode::FlyThrough,
                pos,
            ));
        builder = builder.with(agent);
    }

    let new_entity = builder.build();

    if tethered == Some(true) {
        let tether_leader = server
            .state
            .read_component_cloned::<Is<Rider>>(target)
            .map(|is_rider| is_rider.mount)
            .or_else(|| {
                server
                    .state
                    .read_component_cloned::<Is<VolumeRider>>(target)
                    .and_then(|is_volume_rider| {
                        if let Volume::Entity(uid) = is_volume_rider.pos.kind {
                            Some(uid)
                        } else {
                            None
                        }
                    })
            })
            .or_else(|| server.state.ecs().uid_from_entity(target));
        let tether_follower = server.state.ecs().uid_from_entity(new_entity);

        if let (Some(leader), Some(follower)) = (tether_leader, tether_follower) {
            let tether_length = tether_leader
                .and_then(|uid| server.state.ecs().entity_from_uid(uid))
                .and_then(|e| server.state.read_component_cloned::<comp::Body>(e))
                .map(|b| b.dimensions().y * 1.5 + 1.0)
                .unwrap_or(6.0);
            server
                .state
                .link(Tethered {
                    leader,
                    follower,
                    tether_length,
                })
                .map_err(|_| Content::Plain("Failed to tether entities".to_string()))?;
        } else {
            return Err(Content::Plain("Tether members don't have Uids.".into()));
        }
    }

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::Plain("Spawned a ship".to_string()),
        ),
    );
    Ok(())
}

fn handle_make_volume(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    use comp::body::ship::figuredata::VoxelCollider;

    //let () = parse_cmd_args!(args);
    let pos = position(server, target, "target")?;
    let ship = comp::ship::Body::Volume;
    let sz = parse_cmd_args!(args, u32).unwrap_or(15);
    if !(1..=127).contains(&sz) {
        return Err(Content::localized("command-volume-size-incorrect"));
    };
    let sz = Vec3::broadcast(sz);
    let collider = {
        let terrain = server.state().terrain();
        comp::Collider::Volume(Arc::new(VoxelCollider::from_fn(sz, |rpos| {
            terrain
                .get(pos.0.map(|e| e.floor() as i32) + rpos - sz.map(|e| e as i32) / 2)
                .ok()
                .copied()
                .unwrap_or_else(Block::empty)
        })))
    };
    server
        .state
        .create_ship(
            comp::Pos(pos.0 + Vec3::unit_z() * (50.0 + sz.z as f32 / 2.0)),
            comp::Ori::default(),
            ship,
            move |_| collider,
        )
        .build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized("command-volume-created"),
        ),
    );
    Ok(())
}

fn handle_spawn_campfire(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;
    server
        .state
        .ecs()
        .read_resource::<EventBus<CreateSpecialEntityEvent>>()
        .emit_now(CreateSpecialEntityEvent {
            pos: pos.0,
            entity: SpecialEntity::Waypoint,
        });

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized("command-spawned-campfire"),
        ),
    );
    Ok(())
}

#[cfg(feature = "persistent_world")]
fn handle_clear_persisted_terrain(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let Some(radius) = parse_cmd_args!(args, i32) else {
        return Err(action.help_content());
    };
    // Clamp the radius to prevent accidentally passing too large radiuses
    let radius = radius.clamp(0, 64);

    let pos = position(server, target, "target")?;
    let chunk_key = server.state.terrain().pos_key(pos.0.as_());

    let mut terrain_persistence2 = server
        .state
        .ecs()
        .try_fetch_mut::<crate::terrain_persistence::TerrainPersistence>();
    if let Some(ref mut terrain_persistence) = terrain_persistence2 {
        for offset in Spiral2d::with_radius(radius) {
            let chunk_key = chunk_key + offset;
            terrain_persistence.clear_chunk(chunk_key);
        }

        drop(terrain_persistence2);
        reload_chunks_inner(server, pos.0, Some(radius));

        Ok(())
    } else {
        Err(Content::localized(
            "command-experimental-terrain-persistence-disabled",
        ))
    }
}

#[cfg(not(feature = "persistent_world"))]
fn handle_clear_persisted_terrain(
    _server: &mut Server,
    _client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    Err(Content::localized(
        "command-server-no-experimental-terrain-persistence",
    ))
}

fn handle_safezone(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let range = parse_cmd_args!(args, f32);
    let pos = position(server, target, "target")?;
    server.state.create_safezone(range, pos).build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized("command-spawned-safezone"),
        ),
    );
    Ok(())
}

fn handle_permit_build(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(area_name) = parse_cmd_args!(args, String) {
        let bb_id = area(server, &area_name, "build")?;
        let mut can_build = server.state.ecs().write_storage::<comp::CanBuild>();
        let entry = can_build
            .entry(target)
            .map_err(|_| Content::Plain("Cannot find target entity!".to_string()))?;
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
                    Content::localized_with_args("command-permit-build-given", [(
                        "area",
                        area_name.clone(),
                    )]),
                ),
            );
        }
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized_with_args("command-permit-build-granted", [("area", area_name)]),
            ),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_revoke_build(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(area_name) = parse_cmd_args!(args, String) {
        let bb_id = area(server, &area_name, "build")?;
        let mut can_build = server.state.ecs_mut().write_storage::<comp::CanBuild>();
        if let Some(mut comp_can_build) = can_build.get_mut(target) {
            comp_can_build.build_areas.retain(|&x| x != bb_id);
            drop(can_build);
            if client != target {
                server.notify_client(
                    target,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        Content::localized_with_args("command-revoke-build-recv", [(
                            "area",
                            area_name.clone(),
                        )]),
                    ),
                );
            }
            server.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::localized_with_args("command-revoke-build", [("area", area_name)]),
                ),
            );
            Ok(())
        } else {
            Err(Content::localized("command-no-buid-perms"))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_revoke_build_all(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let ecs = server.state.ecs();

    ecs.write_storage::<comp::CanBuild>().remove(target);
    if client != target {
        server.notify_client(
            target,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized("command-revoke-build-all"),
            ),
        );
    }
    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized("command-revoked-all-build"),
        ),
    );
    Ok(())
}

fn handle_players(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let ecs = server.state.ecs();

    let entity_tuples = (
        &ecs.entities(),
        &ecs.read_storage::<comp::Player>(),
        &ecs.read_storage::<comp::Stats>(),
    );

    // Contruct list of every player currently online
    let mut player_list = String::new();
    for (_, player, stat) in entity_tuples.join() {
        player_list.push_str(&format!(
            "[{}]{}\n",
            player.alias,
            stat.name.as_plain().unwrap_or("<?>")
        ));
    }

    // Show all players currently online
    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized_with_args("players-list-header", [
                (
                    "count",
                    LocalizationArg::from(entity_tuples.join().count() as u64),
                ),
                ("player_list", LocalizationArg::from(player_list)),
            ]),
        ),
    );

    Ok(())
}

fn handle_spawn_portal(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;

    if let (Some(x), Some(y), Some(z), requires_no_aggro, buildup_time) =
        parse_cmd_args!(args, f32, f32, f32, bool, f64)
    {
        let requires_no_aggro = requires_no_aggro.unwrap_or(false);
        let buildup_time = Secs(buildup_time.unwrap_or(7.));
        server
            .state
            .create_teleporter(pos, PortalData {
                target: Vec3::new(x, y, z),
                buildup_time,
                requires_no_aggro,
            })
            .build();

        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::Plain("Spawned portal".to_string()),
            ),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_build(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(mut can_build) = server
        .state
        .ecs()
        .write_storage::<comp::CanBuild>()
        .get_mut(target)
    {
        can_build.enabled ^= true;

        let msg = Content::localized(
            match (
                can_build.enabled,
                server.settings().experimental_terrain_persistence,
            ) {
                (false, _) => "command-set-build-mode-off",
                (true, false) => "command-set-build-mode-on-unpersistent",
                (true, true) => "command-set-build-mode-on-persistent",
            },
        );

        let chat_msg = ServerGeneral::server_msg(ChatType::CommandInfo, msg);
        if client != target {
            server.notify_client(target, chat_msg.clone());
        }
        server.notify_client(client, chat_msg);
        Ok(())
    } else {
        Err(Content::Plain(
            "You do not have permission to build.".into(),
        ))
    }
}

fn get_areas_mut<'l>(kind: &str, state: &'l mut State) -> CmdResult<&'l mut Areas> {
    Ok(match AreaKind::from_str(kind).ok() {
        Some(AreaKind::Build) => state
            .mut_resource::<AreasContainer<BuildArea>>()
            .deref_mut(),
        Some(AreaKind::NoDurability) => state
            .mut_resource::<AreasContainer<NoDurabilityArea>>()
            .deref_mut(),
        None => Err(Content::Plain(format!("Invalid area type '{kind}'")))?,
    })
}

fn handle_area_add(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (
        Some(area_name),
        Some(kind),
        Some(xlo),
        Some(xhi),
        Some(ylo),
        Some(yhi),
        Some(zlo),
        Some(zhi),
    ) = parse_cmd_args!(args, String, String, i32, i32, i32, i32, i32, i32)
    {
        let special_areas = get_areas_mut(&kind, &mut server.state)?;
        let msg = ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::Plain(format!("Created {kind} zone {}", area_name)),
        );
        special_areas
            .insert(area_name, Aabb {
                min: Vec3::new(xlo, ylo, zlo),
                max: Vec3::new(xhi, yhi, zhi),
            })
            .map_err(|area_name| {
                Content::Plain(format!("{kind} zone {} already exists!", area_name))
            })?;
        server.notify_client(client, msg);
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_area_list(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let format_areas = |areas: &Areas, kind: &str| {
        areas
            .area_metas()
            .iter()
            .fold(format!("{kind} areas:"), |acc, (area_name, bb_id)| {
                if let Some(aabb) = areas.areas().get(*bb_id) {
                    format!("{}\n{}: {} to {} ()", acc, area_name, aabb.min, aabb.max,)
                } else {
                    acc
                }
            })
    };
    let build_message = format_areas(
        server.state.mut_resource::<AreasContainer<BuildArea>>(),
        "Build",
    );
    let no_dura_message = format_areas(
        server
            .state
            .mut_resource::<AreasContainer<NoDurabilityArea>>(),
        "Durability free",
    );

    let msg = ServerGeneral::server_msg(
        ChatType::CommandInfo,
        Content::Plain([build_message, no_dura_message].join("\n")),
    );

    server.notify_client(client, msg);
    Ok(())
}

fn handle_area_remove(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(area_name), Some(kind)) = parse_cmd_args!(args, String, String) {
        let areas = get_areas_mut(&kind, &mut server.state)?;

        areas.remove(&area_name).map_err(|err| match err {
            SpecialAreaError::Reserved => Content::Plain(format!(
                "Special area is reserved and cannot be removed: {}",
                area_name
            )),
            SpecialAreaError::NotFound => {
                Content::Plain(format!("No such build area {}", area_name))
            },
        })?;
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::Plain(format!("Removed {kind} zone {area_name}")),
            ),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn parse_alignment(owner: Uid, alignment: &str) -> CmdResult<Alignment> {
    match alignment {
        "wild" => Ok(Alignment::Wild),
        "enemy" => Ok(Alignment::Enemy),
        "npc" => Ok(Alignment::Npc),
        "pet" => Ok(comp::Alignment::Owned(owner)),
        _ => Err(Content::localized_with_args("command-invalid-alignment", [
            ("alignment", alignment),
        ])),
    }
}

fn handle_kill_npcs(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let (radius, options) = parse_cmd_args!(args, f32, String);
    let kill_pets = if let Some(kill_option) = options {
        kill_option.contains("--also-pets")
    } else {
        false
    };

    let position = radius
        .map(|_| position(server, target, "target"))
        .transpose()?;

    let to_kill = {
        let ecs = server.state.ecs();
        let entities = ecs.entities();
        let positions = ecs.write_storage::<comp::Pos>();
        let healths = ecs.write_storage::<comp::Health>();
        let players = ecs.read_storage::<comp::Player>();
        let alignments = ecs.read_storage::<Alignment>();
        let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
        let mut rtsim = ecs.write_resource::<crate::rtsim::RtSim>();
        let spatial_grid;

        let mut iter_a;
        let mut iter_b;

        let iter: &mut dyn Iterator<
            Item = (
                EcsEntity,
                &comp::Health,
                (),
                Option<&comp::Alignment>,
                &comp::Pos,
            ),
        > = if let (Some(radius), Some(position)) = (radius, position) {
            spatial_grid = ecs.read_resource::<CachedSpatialGrid>();
            iter_a = spatial_grid
                .0
                .in_circle_aabr(position.0.xy(), radius)
                .filter_map(|entity| {
                    (
                        &entities,
                        &healths,
                        !&players,
                        alignments.maybe(),
                        &positions,
                    )
                        .lend_join()
                        .get(entity, &entities)
                })
                .filter(move |(_, _, _, _, pos)| {
                    pos.0.distance_squared(position.0) <= radius.powi(2)
                });

            &mut iter_a as _
        } else {
            iter_b = (
                &entities,
                &healths,
                !&players,
                alignments.maybe(),
                &positions,
            )
                .join();

            &mut iter_b as _
        };

        iter.filter_map(|(entity, _health, (), alignment, pos)| {
            let should_kill = kill_pets
                || if let Some(Alignment::Owned(owned)) = alignment {
                    ecs.entity_from_uid(*owned)
                        .is_none_or(|owner| !players.contains(owner))
                } else {
                    true
                };

            if should_kill {
                if let Some(rtsim_entity) = rtsim_entities.get(entity).copied() {
                    rtsim.hook_rtsim_actor_death(
                        &ecs.read_resource::<Arc<world::World>>(),
                        ecs.read_resource::<world::IndexOwned>().as_index_ref(),
                        Actor::Npc(rtsim_entity.0),
                        Some(pos.0),
                        None,
                    );
                }
                Some(entity)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
    };
    let count = to_kill.len();
    for entity in to_kill {
        // Directly remove entities instead of modifying health to avoid loot drops.
        if let Err(e) = server.state.delete_entity_recorded(entity) {
            error!(?e, ?entity, "Failed to delete entity");
        }
    }
    let text = if count > 0 {
        format!("Destroyed {} NPCs.", count)
    } else {
        "No NPCs on server.".to_string()
    };

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, Content::Plain(text)),
    );

    Ok(())
}

enum KitEntry {
    Spec(KitSpec),
    Item(Item),
}

impl From<KitSpec> for KitEntry {
    fn from(spec: KitSpec) -> Self { Self::Spec(spec) }
}

fn handle_kit(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    use common::cmd::KitManifest;

    let notify = |server: &mut Server, kit_name: &str| {
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::Plain(format!("Gave kit: {}", kit_name)),
            ),
        );
    };
    let name = parse_cmd_args!(args, String).ok_or_else(|| action.help_content())?;

    match name.as_str() {
        "all" => {
            // This can't fail, we have tests
            let items = all_items_expect();
            let total = items.len();

            let res = push_kit(
                items.into_iter().map(|item| (KitEntry::Item(item), 1)),
                total,
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
                .map_err(|_| {
                    Content::Plain(format!(
                        "Could not load manifest file {}",
                        KIT_MANIFEST_PATH
                    ))
                })?;

            let kit = kits
                .0
                .get(kit_name)
                .ok_or(Content::Plain(format!("Kit '{}' not found", kit_name)))?;

            let res = push_kit(
                kit.iter()
                    .map(|(item_id, quantity)| (item_id.clone().into(), *quantity)),
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

fn push_kit<I>(kit: I, count: usize, server: &mut Server, target: EcsEntity) -> CmdResult<()>
where
    I: Iterator<Item = (KitEntry, u32)>,
{
    if let (Some(mut target_inventory), mut target_inv_update) = (
        server
            .state()
            .ecs()
            .write_storage::<Inventory>()
            .get_mut(target),
        server.state.ecs().write_storage::<comp::InventoryUpdate>(),
    ) {
        // TODO: implement atomic `insert_all_or_nothing` on Inventory
        if target_inventory.free_slots() < count {
            return Err(Content::localized("command-kit-not-enough-slots"));
        }

        for (item_id, quantity) in kit {
            push_item(item_id, quantity, server, &mut |item| {
                let res = target_inventory.push(item);
                let _ = target_inv_update.insert(
                    target,
                    comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Debug),
                );

                res
            })?;
        }

        Ok(())
    } else {
        Err(Content::localized("command-kit-inventory-unavailable"))
    }
}

fn push_item(
    item_id: KitEntry,
    quantity: u32,
    server: &Server,
    push: &mut dyn FnMut(Item) -> Result<(), (Item, Option<NonZeroU32>)>,
) -> CmdResult<()> {
    let items = match item_id {
        KitEntry::Spec(KitSpec::Item(item_id)) => vec![
            Item::new_from_asset(&item_id)
                .map_err(|_| Content::Plain(format!("Unknown item: {:#?}", item_id)))?,
        ],
        KitEntry::Spec(KitSpec::ModularWeaponSet {
            tool,
            material,
            hands,
        }) => comp::item::modular::generate_weapons(tool, material, hands)
            .map_err(|err| Content::Plain(format!("{:#?}", err)))?,
        KitEntry::Spec(KitSpec::ModularWeaponRandom {
            tool,
            material,
            hands,
        }) => {
            let mut rng = rand::thread_rng();
            vec![
                comp::item::modular::random_weapon(tool, material, hands, &mut rng)
                    .map_err(|err| Content::Plain(format!("{:#?}", err)))?,
            ]
        },
        KitEntry::Item(item) => vec![item],
    };

    let mut res = Ok(());
    for mut item in items {
        // Either push stack or push one by one.
        if item.is_stackable() {
            // FIXME: in theory, this can fail,
            // but we don't have stack sizes yet.
            let _ = item.set_amount(quantity);
            res = push(item);
        } else {
            let ability_map = server.state.ecs().read_resource::<AbilityMap>();
            let msm = server.state.ecs().read_resource::<MaterialStatManifest>();

            for _ in 0..quantity {
                res = push(item.duplicate(&ability_map, &msm));

                if res.is_err() {
                    break;
                }
            }
        }

        // I think it's possible to pick-up item during this loop
        // and fail into case where you had space but now you don't?
        if res.is_err() {
            return Err(Content::localized("command-inventory-cant-fit-item"));
        }
    }

    Ok(())
}

fn handle_object(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let obj_type = parse_cmd_args!(args, String);

    let pos = position(server, target, "target")?;
    let ori = server
        .state
        .ecs()
        .read_storage::<comp::Ori>()
        .get(target)
        .copied()
        .ok_or_else(|| Content::Plain("Cannot get orientation for target".to_string()))?;
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
                Content::Plain(format!(
                    "Spawned: {}",
                    obj_str_res.unwrap_or("<Unknown object>")
                )),
            ),
        );
        Ok(())
    } else {
        Err(Content::Plain("Object not found!".into()))
    }
}

fn handle_outcome(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let mut i = 0;

    macro_rules! arg {
        () => {
            args.get(i).map(|r| {
                i += 1;
                r
            })
        };
        ($err:expr) => {
            arg!().ok_or_else(|| Content::Key(($err).to_string()))
        };
    }

    let target_pos = server
        .state
        .read_component_copied::<comp::Pos>(target)
        .unwrap_or(comp::Pos(Vec3::zero()));
    let target_uid = server
        .state
        .read_component_copied::<Uid>(target)
        .expect("All entities should have uids");

    macro_rules! vec_arg {
        () => {{
            let old_i = i;
            let pos = arg!().and_then(|arg| {
                let x = arg.parse().ok()?;
                let y = arg!()?.parse().ok()?;
                let z = arg!()?.parse().ok()?;

                Some(Vec3::new(x, y, z))
            });

            #[allow(unused_assignments)]
            if let Some(pos) = pos {
                pos
            } else {
                i = old_i;
                Vec3::default()
            }
        }};
    }

    macro_rules! pos_arg {
        () => {{
            let old_i = i;
            let pos = arg!().and_then(|arg| {
                let x = arg.parse().ok()?;
                let y = arg!()?.parse().ok()?;
                let z = arg!()?.parse().ok()?;

                Some(Vec3::new(x, y, z))
            });

            #[allow(unused_assignments)]
            if let Some(pos) = pos {
                pos
            } else {
                i = old_i;
                target_pos.0.as_()
            }
        }};
    }

    macro_rules! parse {
        ($err:expr, $expr:expr) => {
            arg!()
                .and_then($expr)
                .ok_or_else(|| Content::Key(($err).to_string()))
        };
        ($err:expr) => {
            parse!($err, |arg| arg.parse().ok())
        };
    }

    macro_rules! body_arg {
        () => {{ parse!("command-outcome-expected_body_arg").map(|npc::NpcBody(_, mut body)| body()) }};
    }

    macro_rules! uid_arg {
        () => {{
            parse!("command-outcome-expected_entity_arg").and_then(|entity| {
                let entity = get_entity_target(entity, server)?;
                Ok(server
                    .state()
                    .read_component_copied::<Uid>(entity)
                    .expect("All entities have uids"))
            })
        }};
    }

    macro_rules! parse_or_default {
        ($default:expr, @$expr:expr) => {{
            let old_i = i;
            let f = arg!().and_then($expr);

            #[allow(unused_assignments)]
            if let Some(f) = f {
                f
            } else {
                i = old_i;
                $default
            }
        }};
        (@$expr:expr) => {{ parse_or_default!(Default::default(), @$expr) }};
        ($default:expr) => {{ parse_or_default!($default, @|arg| arg.parse().ok()) }};
        () => {
            parse_or_default!(Default::default())
        };
    }

    let mut rng = rand::thread_rng();

    let outcome = arg!("command-outcome-variant_expected")?;

    let outcome = match outcome.as_str() {
        "Explosion" => Outcome::Explosion {
            pos: pos_arg!(),
            power: parse_or_default!(1.0),
            radius: parse_or_default!(1.0),
            is_attack: parse_or_default!(),
            reagent: parse_or_default!(@|arg| comp::item::Reagent::from_str(arg).ok().map(Some)),
        },
        "Lightning" => Outcome::Lightning { pos: pos_arg!() },
        "ProjectileShot" => Outcome::ProjectileShot {
            pos: pos_arg!(),
            body: body_arg!()?,
            vel: vec_arg!(),
        },
        "ProjectileHit" => Outcome::ProjectileHit {
            pos: pos_arg!(),
            body: body_arg!()?,
            vel: vec_arg!(),
            source: uid_arg!().ok(),
            target: uid_arg!().ok(),
        },
        "Beam" => Outcome::Beam {
            pos: pos_arg!(),
            specifier: parse!("command-outcome-expected_frontent_specifier", |arg| {
                comp::beam::FrontendSpecifier::from_str(arg).ok()
            })?,
        },
        "ExpChange" => Outcome::ExpChange {
            uid: uid_arg!().unwrap_or(target_uid),
            exp: parse!("command-outcome-expected_integer")?,
            xp_pools: {
                let mut hashset = HashSet::new();
                while let Some(arg) = arg!() {
                    hashset.insert(ron::from_str(arg).map_err(|_| {
                        Content::Key("command-outcome-expected_skill_group_kind".to_string())
                    })?);
                }
                hashset
            },
        },
        "SkillPointGain" => Outcome::SkillPointGain {
            uid: uid_arg!().unwrap_or(target_uid),
            skill_tree: arg!("command-outcome-expected_skill_group_kind").and_then(|arg| {
                ron::from_str(arg).map_err(|_| {
                    Content::Key("command-outcome-expected_skill_group_kind".to_string())
                })
            })?,
            total_points: parse!("Expected an integer amount of points")?,
        },
        "ComboChange" => Outcome::ComboChange {
            uid: uid_arg!().unwrap_or(target_uid),
            combo: parse!("command-outcome-expected_integer")?,
        },
        "BreakBlock" => Outcome::BreakBlock {
            pos: pos_arg!(),
            color: Some(Rgb::from(vec_arg!())),
            tool: None,
        },
        "SummonedCreature" => Outcome::SummonedCreature {
            pos: pos_arg!(),
            body: body_arg!()?,
        },
        "HealthChange" => Outcome::HealthChange {
            pos: pos_arg!(),
            info: common::outcome::HealthChangeInfo {
                amount: parse_or_default!(),
                precise: parse_or_default!(),
                target: uid_arg!().unwrap_or(target_uid),
                by: uid_arg!().map(common::combat::DamageContributor::Solo).ok(),
                cause: None,
                instance: rng.gen(),
            },
        },
        "Death" => Outcome::Death { pos: pos_arg!() },
        "Block" => Outcome::Block {
            pos: pos_arg!(),
            parry: parse_or_default!(),
            uid: uid_arg!().unwrap_or(target_uid),
        },
        "PoiseChange" => Outcome::PoiseChange {
            pos: pos_arg!(),
            state: parse_or_default!(comp::PoiseState::Normal, @|arg| comp::PoiseState::from_str(arg).ok()),
        },
        "GroundSlam" => Outcome::GroundSlam { pos: pos_arg!() },
        "IceSpikes" => Outcome::IceSpikes { pos: pos_arg!() },
        "IceCrack" => Outcome::IceCrack { pos: pos_arg!() },
        "FlashFreeze" => Outcome::FlashFreeze { pos: pos_arg!() },
        "Steam" => Outcome::Steam { pos: pos_arg!() },
        "LaserBeam" => Outcome::LaserBeam { pos: pos_arg!() },
        "CyclopsCharge" => Outcome::CyclopsCharge { pos: pos_arg!() },
        "FlamethrowerCharge" => Outcome::FlamethrowerCharge { pos: pos_arg!() },
        "FuseCharge" => Outcome::FuseCharge { pos: pos_arg!() },
        "TerracottaStatueCharge" => Outcome::TerracottaStatueCharge { pos: pos_arg!() },
        "SurpriseEgg" => Outcome::SurpriseEgg { pos: pos_arg!() },
        "Utterance" => Outcome::Utterance {
            pos: pos_arg!(),
            body: body_arg!()?,
            kind: parse_or_default!(comp::UtteranceKind::Greeting, @|arg| comp::UtteranceKind::from_str(arg).ok()),
        },
        "Glider" => Outcome::Glider {
            pos: pos_arg!(),
            wielded: parse_or_default!(true),
        },
        "SpriteDelete" => Outcome::SpriteDelete {
            pos: pos_arg!(),
            sprite: parse!("command-outcome-expected_sprite_kind", |arg| {
                SpriteKind::try_from(arg.as_str()).ok()
            })?,
        },
        "SpriteUnlocked" => Outcome::SpriteUnlocked { pos: pos_arg!() },
        "FailedSpriteUnlock" => Outcome::FailedSpriteUnlock { pos: pos_arg!() },
        "Whoosh" => Outcome::Whoosh { pos: pos_arg!() },
        "Swoosh" => Outcome::Swoosh { pos: pos_arg!() },
        "Slash" => Outcome::Slash { pos: pos_arg!() },
        "FireShockwave" => Outcome::FireShockwave { pos: pos_arg!() },
        "GroundDig" => Outcome::GroundDig { pos: pos_arg!() },
        "PortalActivated" => Outcome::PortalActivated { pos: pos_arg!() },
        "TeleportedByPortal" => Outcome::TeleportedByPortal { pos: pos_arg!() },
        "FromTheAshes" => Outcome::FromTheAshes { pos: pos_arg!() },
        "ClayGolemDash" => Outcome::ClayGolemDash { pos: pos_arg!() },
        "Bleep" => Outcome::Bleep { pos: pos_arg!() },
        "Charge" => Outcome::Charge { pos: pos_arg!() },
        "HeadLost" => Outcome::HeadLost {
            uid: uid_arg!().unwrap_or(target_uid),
            head: parse_or_default!(),
        },
        "Splash" => Outcome::Splash {
            vel: vec_arg!(),
            pos: pos_arg!(),
            mass: parse_or_default!(1.0),
            kind: parse_or_default!(
                comp::fluid_dynamics::LiquidKind::Water,
                @|arg| comp::fluid_dynamics::LiquidKind::from_str(arg).ok()
            ),
        },
        _ => {
            return Err(Content::localized_with_args(
                "command-outcome-invalid_outcome",
                [("outcome", Content::Plain(outcome.to_string()))],
            ));
        },
    };

    server
        .state()
        .ecs()
        .read_resource::<EventBus<Outcome>>()
        .emit_now(outcome);

    Ok(())
}

fn handle_light(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let (opt_r, opt_g, opt_b, opt_x, opt_y, opt_z, opt_s) =
        parse_cmd_args!(args, f32, f32, f32, f32, f32, f32, f32);

    let mut light_emitter = LightEmitter::default();
    let mut light_offset_opt = None;

    if let (Some(r), Some(g), Some(b)) = (opt_r, opt_g, opt_b) {
        if r < 0.0 || g < 0.0 || b < 0.0 {
            return Err(Content::Plain(
                "cr, cg and cb values mustn't be negative.".into(),
            ));
        }

        let r = r.clamp(0.0, 1.0);
        let g = g.clamp(0.0, 1.0);
        let b = b.clamp(0.0, 1.0);
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
        // TODO: I don't think we intend to add this component to non-client entities?
        .with(comp::ForceUpdate::forced())
        .with(light_emitter);
    if let Some(light_offset) = light_offset_opt {
        builder.with(light_offset).build();
    } else {
        builder.build();
    }
    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::Plain("Spawned object.".to_string()),
        ),
    );
    Ok(())
}

fn handle_lantern(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(s), r, g, b) = parse_cmd_args!(args, f32, f32, f32, f32) {
        if let Some(mut light) = server
            .state
            .ecs()
            .write_storage::<LightEmitter>()
            .get_mut(target)
        {
            light.strength = s.clamp(0.1, 10.0);
            if let (Some(r), Some(g), Some(b)) = (r, g, b) {
                light.col = (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)).into();
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        Content::localized("command-lantern-adjusted-strength-color"),
                    ),
                )
            } else {
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        Content::localized("command-lantern-adjusted-strength"),
                    ),
                )
            }
            Ok(())
        } else {
            Err(Content::localized("command-lantern-unequiped"))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_explosion(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let power = parse_cmd_args!(args, f32).unwrap_or(8.0);

    const MIN_POWER: f32 = 0.0;
    const MAX_POWER: f32 = 512.0;

    if power > MAX_POWER {
        return Err(Content::localized_with_args(
            "command-explosion-power-too-high",
            [("power", MAX_POWER.to_string())],
        ));
    } else if power <= MIN_POWER {
        return Err(Content::localized_with_args(
            "command-explosion-power-too-low",
            [("power", MIN_POWER.to_string())],
        ));
    }

    let pos = position(server, target, "target")?;
    let owner = server
        .state
        .ecs()
        .read_storage::<Uid>()
        .get(target)
        .copied();
    server.state.emit_event_now(ExplosionEvent {
        pos: pos.0,
        explosion: Explosion {
            effects: vec![
                RadiusEffect::Entity(Effect::Damage(Damage {
                    source: DamageSource::Explosion,
                    kind: DamageKind::Energy,
                    value: 100.0 * power,
                })),
                RadiusEffect::TerrainDestruction(power, Rgb::black()),
            ],
            radius: 3.0 * power,
            reagent: None,
            min_falloff: 0.0,
        },
        owner,
    });
    Ok(())
}

fn handle_set_waypoint(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let pos = position(server, target, "target")?;
    let time = *server.state.mut_resource::<Time>();
    let location_name = server
        .world()
        .get_location_name(server.index.as_index_ref(), pos.0.xy().as_::<i32>());

    insert_or_replace_component(
        server,
        target,
        comp::Waypoint::temp_new(pos.0, time),
        "target",
    )?;
    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized("command-set-waypoint-result"),
        ),
    );

    if let Some(location_name) = location_name {
        server.notify_client(
            target,
            ServerGeneral::Notification(Notification::WaypointSaved { location_name }),
        );
    } else {
        error!(
            "Failed to get location name for waypoint. Client was not notified of new waypoint."
        );
    }

    Ok(())
}

fn handle_spawn_wiring(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let mut pos = position(server, target, "target")?;
    pos.0.x += 3.0;

    let mut outputs1 = HashMap::new();
    outputs1.insert("button".to_string(), OutputFormula::OnCollide {
        value: 1.0,
    });

    // Create the first element of the circuit.
    // This is a coin body. This element does not have any inputs or actions.
    // Instead there is one output. When there is a collision with this element the
    // value of 1.0 will be sent as an input with the "button" label. Any
    // element with an `Input` for the name "button" can use this value as an
    // input. The threshold does not matter as there are no effects for this
    // element.
    let builder1 = server
        .state
        .create_wiring(pos, comp::object::Body::Pebble, WiringElement {
            inputs: HashMap::new(),
            outputs: outputs1,
            actions: Vec::new(),
        })
        .with(comp::Density(100_f32));
    let ent1 = builder1.build();

    pos.0.x += 3.0;
    // The second element has no elements in the `inputs` field to start with. When
    // the circuit runs, the input as specified by the `Input` OutputFormula is
    // added to the inputs. The next tick the effect(s) are applied based on the
    // input value.
    let builder2 = server
        .state
        .create_wiring(pos, comp::object::Body::Pebble, WiringElement {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            actions: vec![WiringAction {
                formula: OutputFormula::Input {
                    name: String::from("button"),
                },
                threshold: 0.0,
                effects: vec![WiringActionEffect::SetLight {
                    r: OutputFormula::Input {
                        name: String::from("button"),
                    },
                    g: OutputFormula::Input {
                        name: String::from("button"),
                    },
                    b: OutputFormula::Input {
                        name: String::from("button"),
                    },
                }],
            }],
        })
        .with(comp::Density(100_f32));
    let ent2 = builder2.build();

    pos.0.x += 3.0;
    let builder3 = server
        .state
        .create_wiring(pos, comp::object::Body::TrainingDummy, WiringElement {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            actions: Vec::new(),
        })
        .with(comp::Density(comp::object::Body::TrainingDummy.density().0))
        .with(Circuit::new(vec![Wire {
            input: WireNode::new(ent1, "button".to_string()),
            output: WireNode::new(ent2, "button".to_string()),
        }]));
    builder3.build();

    server.notify_client(
        client,
        ServerGeneral::server_msg(ChatType::CommandInfo, Content::Plain("Wire".to_string())),
    );
    Ok(())
}

fn handle_adminify(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(alias), desired_role) = parse_cmd_args!(args, String, String) {
        let desired_role = if let Some(mut desired_role) = desired_role {
            desired_role.make_ascii_lowercase();
            Some(match &*desired_role {
                "admin" => AdminRole::Admin,
                "moderator" => AdminRole::Moderator,
                _ => {
                    return Err(action.help_content());
                },
            })
        } else {
            None
        };
        let (player, player_uuid) = find_alias(server.state.ecs(), &alias, true)?;
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
            Content::localized("command-adminify-reassign-to-above"),
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
            return Err(Content::localized(
                "command-adminify-assign-higher-than-own",
            ));
        }

        let mut admin_storage = server.state.ecs().write_storage::<comp::Admin>();
        let entry = admin_storage
            .entry(player)
            .map_err(|_| Content::localized("command-adminify-cannot-find-player"))?;
        match (entry, desired_role) {
            (StorageEntry::Vacant(_), None) => {
                return Err(Content::localized("command-adminify-already-has-no-role"));
            },
            (StorageEntry::Occupied(o), None) => {
                let old_role = o.remove().0;
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        Content::localized_with_args("command-adminify-removed-role", [
                            ("player", alias),
                            ("role", format!("{:?}", old_role)),
                        ]),
                    ),
                );
            },
            (entry, Some(desired_role)) => {
                let key = match entry
                    .replace(comp::Admin(desired_role))
                    .map(|old_admin| old_admin.0.cmp(&desired_role))
                {
                    Some(Ordering::Equal) => {
                        return Err(Content::localized("command-adminify-already-has-role"));
                    },
                    Some(Ordering::Greater) => "command-adminify-role-downgraded",
                    Some(Ordering::Less) | None => "command-adminify-role-upgraded",
                };
                server.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        Content::localized_with_args(key, [
                            ("player", alias),
                            ("role", format!("{:?}", desired_role)),
                        ]),
                    ),
                );
            },
        };

        // Notify the client that its role has been updated
        server.notify_client(player, ServerGeneral::SetPlayerRole(desired_role));

        if server
            .state
            .ecs()
            .read_storage::<Client>()
            .get(player)
            .is_some_and(|client| client.client_type.emit_login_events())
        {
            // Update player list so the player shows up as moderator in client chat.
            //
            // NOTE: We deliberately choose not to differentiate between moderators and
            // administrators in the player list.
            let is_moderator = desired_role.is_some();
            let msg =
                ServerGeneral::PlayerListUpdate(PlayerListUpdate::Moderator(uid, is_moderator));
            server.state.notify_players(msg);
        }
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_tell(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;
    can_send_message(target, server)?;

    if let (Some(alias), message_opt) = parse_cmd_args!(args, String, ..Vec<String>) {
        let ecs = server.state.ecs();
        let player = find_alias(ecs, &alias, false)?.0;

        if player == target {
            return Err(Content::localized("command-tell-to-yourself"));
        }
        let target_uid = uid(server, target, "target")?;
        let player_uid = uid(server, player, "player")?;
        let mode = comp::ChatMode::Tell(player_uid);
        insert_or_replace_component(server, target, mode.clone(), "target")?;
        if !message_opt.is_empty() {
            let msg = Content::Plain(message_opt.join(" "));
            server
                .state
                .send_chat(mode.to_msg(target_uid, msg, None)?, false);
        };
        server.notify_client(target, ServerGeneral::ChatMode(mode));
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_faction(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;
    can_send_message(target, server)?;

    let factions = server.state.ecs().read_storage();
    if let Some(comp::Faction(faction)) = factions.get(target) {
        let mode = comp::ChatMode::Faction(faction.to_string());
        drop(factions);
        insert_or_replace_component(server, target, mode.clone(), "target")?;
        let msg = args.join(" ");
        if !msg.is_empty() {
            if let Some(uid) = server.state.ecs().read_storage().get(target) {
                server
                    .state
                    .send_chat(mode.to_msg(*uid, Content::Plain(msg), None)?, false);
            }
        }
        server.notify_client(target, ServerGeneral::ChatMode(mode));
        Ok(())
    } else {
        Err(Content::localized("command-faction-join"))
    }
}

fn handle_group(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;
    can_send_message(target, server)?;

    let groups = server.state.ecs().read_storage::<comp::Group>();
    if let Some(group) = groups.get(target).copied() {
        let mode = comp::ChatMode::Group;
        drop(groups);
        insert_or_replace_component(server, target, mode.clone(), "target")?;
        let msg = args.join(" ");
        if !msg.is_empty() {
            if let Some(uid) = server.state.ecs().read_storage().get(target) {
                server
                    .state
                    .send_chat(mode.to_msg(*uid, Content::Plain(msg), Some(group))?, false);
            }
        }
        server.notify_client(target, ServerGeneral::ChatMode(mode));
        Ok(())
    } else {
        Err(Content::localized("command-group-join"))
    }
}

fn handle_group_invite(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    // Very hypothetical case: Prevent an admin from running /group_invite using
    // /sudo on a moderator who is currently in silent spectator.
    can_send_message(target, server)?;

    if let Some(target_alias) = parse_cmd_args!(args, String) {
        let target_player = find_alias(server.state.ecs(), &target_alias, false)?.0;
        let uid = uid(server, target_player, "player")?;

        server
            .state
            .emit_event_now(InitiateInviteEvent(target, uid, InviteKind::Group));

        if client != target {
            server.notify_client(
                target,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::localized_with_args("command-group_invite-invited-to-your-group", [(
                        "player",
                        target_alias.to_owned(),
                    )]),
                ),
            );
        }

        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized_with_args("command-group_invite-invited-to-group", [(
                    "player",
                    target_alias.to_owned(),
                )]),
            ),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_group_kick(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    // Checking if leader is already done in group_manip
    if let Some(target_alias) = parse_cmd_args!(args, String) {
        let target_player = find_alias(server.state.ecs(), &target_alias, false)?.0;
        let uid = uid(server, target_player, "player")?;

        server
            .state
            .emit_event_now(GroupManipEvent(target, comp::GroupManip::Kick(uid)));
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_group_leave(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    server
        .state
        .emit_event_now(GroupManipEvent(target, comp::GroupManip::Leave));
    Ok(())
}

fn handle_group_promote(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    // Checking if leader is already done in group_manip
    if let Some(target_alias) = parse_cmd_args!(args, String) {
        let target_player = find_alias(server.state.ecs(), &target_alias, false)?.0;
        let uid = uid(server, target_player, "player")?;

        server
            .state
            .emit_event_now(GroupManipEvent(target, comp::GroupManip::AssignLeader(uid)));
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_reset_recipes(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(mut inventory) = server
        .state
        .ecs()
        .write_storage::<comp::Inventory>()
        .get_mut(target)
    {
        inventory.reset_recipes();
        server.notify_client(target, ServerGeneral::UpdateRecipes);
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_region(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;
    can_send_message(target, server)?;

    let mode = comp::ChatMode::Region;
    insert_or_replace_component(server, target, mode.clone(), "target")?;
    let msg = args.join(" ");
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(target) {
            server
                .state
                .send_chat(mode.to_msg(*uid, Content::Plain(msg), None)?, false);
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
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;
    can_send_message(target, server)?;

    let mode = comp::ChatMode::Say;
    insert_or_replace_component(server, target, mode.clone(), "target")?;
    let msg = args.join(" ");
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(target) {
            server
                .state
                .send_chat(mode.to_msg(*uid, Content::Plain(msg), None)?, false);
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
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;
    can_send_message(target, server)?;

    let mode = comp::ChatMode::World;
    insert_or_replace_component(server, target, mode.clone(), "target")?;
    let msg = args.join(" ");
    if !msg.is_empty() {
        if let Some(uid) = server.state.ecs().read_storage().get(target) {
            server
                .state
                .send_chat(mode.to_msg(*uid, Content::Plain(msg), None)?, false);
        }
    }
    server.notify_client(target, ServerGeneral::ChatMode(mode));
    Ok(())
}

fn handle_join_faction(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    no_sudo(client, target)?;
    let emit_join_message = server
        .state
        .ecs()
        .read_storage::<Client>()
        .get(target)
        .is_some_and(|client| client.client_type.emit_login_events());

    let players = server.state.ecs().read_storage::<comp::Player>();
    if let Some(alias) = players.get(target).map(|player| player.alias.clone()) {
        drop(players);
        let (faction_leave, mode) = if let Some(faction) = parse_cmd_args!(args, String) {
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

            if emit_join_message {
                server.state.send_chat(
                    // TODO: Localise
                    ChatType::FactionMeta(faction.clone())
                        .into_plain_msg(format!("[{}] joined faction ({})", alias, faction)),
                    false,
                );
            }
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
        if let Some(faction) = faction_leave
            && emit_join_message
        {
            server.state.send_chat(
                // TODO: Localise
                ChatType::FactionMeta(faction.clone())
                    .into_plain_msg(format!("[{}] left faction ({})", alias, faction)),
                false,
            );
        }
        server.notify_client(target, ServerGeneral::ChatMode(mode));
        Ok(())
    } else {
        Err(Content::Plain("Could not find your player alias".into()))
    }
}

fn handle_death_effect(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let mut args = args.into_iter();

    let Some(effect_str) = args.next() else {
        return Err(action.help_content());
    };

    let effect = match effect_str.as_str() {
        "transform" => {
            let entity_config = args.next().ok_or(action.help_content())?;

            // We don't actually use this loaded config for anything, this is just a check
            // to ensure loading succeeds later on.
            if EntityConfig::load(&entity_config).is_err() {
                return Err(Content::localized_with_args(
                    "command-entity-load-failed",
                    [("config", entity_config)],
                ));
            }

            combat::DeathEffect::Transform {
                entity_spec: entity_config,
                allow_players: true,
            }
        },
        unknown_effect => {
            return Err(Content::localized_with_args(
                "command-death_effect-unknown",
                [("effect", unknown_effect)],
            ));
        },
    };

    let mut death_effects = server.state.ecs().write_storage::<combat::DeathEffects>();

    if let Some(death_effects) = death_effects.get_mut(target) {
        death_effects.0.push(effect);
    } else {
        death_effects
            .insert(target, combat::DeathEffects(vec![effect]))
            .unwrap();
    }

    Ok(())
}

#[cfg(not(feature = "worldgen"))]
fn handle_debug_column(
    _server: &mut Server,
    _client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    Err(Content::Plain(
        "Unsupported without worldgen enabled".into(),
    ))
}

#[cfg(feature = "worldgen")]
fn handle_debug_column(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let sim = server.world.sim();
    let calendar = (*server.state.ecs().read_resource::<Calendar>()).clone();
    let sampler = server.world.sample_columns();
    let wpos = if let (Some(x), Some(y)) = parse_cmd_args!(args, i32, i32) {
        Vec2::new(x, y)
    } else {
        let pos = position(server, target, "target")?;
        // FIXME: Deal with overflow, if needed.
        pos.0.xy().map(|x| x as i32)
    };
    let msg_generator = |calendar| {
        let alt = sim.get_interpolated(wpos, |chunk| chunk.alt)?;
        let basement = sim.get_interpolated(wpos, |chunk| chunk.basement)?;
        let water_alt = sim.get_interpolated(wpos, |chunk| chunk.water_alt)?;
        let chaos = sim.get_interpolated(wpos, |chunk| chunk.chaos)?;
        let temp = sim.get_interpolated(wpos, |chunk| chunk.temp)?;
        let humidity = sim.get_interpolated(wpos, |chunk| chunk.humidity)?;
        let rockiness = sim.get_interpolated(wpos, |chunk| chunk.rockiness)?;
        let tree_density = sim.get_interpolated(wpos, |chunk| chunk.tree_density)?;
        let spawn_rate = sim.get_interpolated(wpos, |chunk| chunk.spawn_rate)?;
        let chunk_pos = wpos.wpos_to_cpos();
        let chunk = sim.get(chunk_pos)?;
        let col = sampler.get((wpos, server.index.as_index_ref(), Some(calendar)))?;
        let gradient = sim.get_gradient_approx(chunk_pos)?;
        let downhill = chunk.downhill;
        let river = &chunk.river;
        let flux = chunk.flux;
        let path = chunk.path;
        let cliff_height = chunk.cliff_height;

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
spawn_rate {:?}
path {:?}
cliff_height {:?} "#,
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
            spawn_rate,
            path,
            cliff_height,
        ))
    };
    if let Some(s) = msg_generator(&calendar) {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, Content::Plain(s)),
        );
        Ok(())
    } else {
        Err(Content::Plain("Not a pre-generated chunk.".into()))
    }
}

#[cfg(not(feature = "worldgen"))]
fn handle_debug_ways(
    _server: &mut Server,
    _client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    Err(Content::Plain(
        "Unsupported without worldgen enabled".into(),
    ))
}

#[cfg(feature = "worldgen")]
fn handle_debug_ways(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let sim = server.world.sim();
    let wpos = if let (Some(x), Some(y)) = parse_cmd_args!(args, i32, i32) {
        Vec2::new(x, y)
    } else {
        let pos = position(server, target, "target")?;
        // FIXME: Deal with overflow, if needed.
        pos.0.xy().map(|x| x as i32)
    };
    let msg_generator = || {
        let chunk_pos = wpos.wpos_to_cpos();
        let mut ret = String::new();
        for delta in LOCALITY {
            let pos = chunk_pos + delta;
            let chunk = sim.get(pos)?;
            writeln!(ret, "{:?}: {:?}", pos, chunk.path).ok()?;
        }
        Some(ret)
    };
    if let Some(s) = msg_generator() {
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, Content::Plain(s)),
        );
        Ok(())
    } else {
        Err(Content::Plain("Not a pre-generated chunk.".into()))
    }
}

fn handle_disconnect_all_players(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let client_uuid = uuid(server, client, "client")?;
    // Make sure temporary mods/admins can't run this command.
    let _role = real_role(server, client_uuid, "role")?;

    if parse_cmd_args!(args, String).as_deref() != Some("confirm") {
        return Err(Content::localized("command-disconnectall-confirm"));
    }

    let ecs = server.state.ecs();
    let players = &ecs.read_storage::<comp::Player>();

    // TODO: This logging and verification of admin commands would be better moved
    // to a more generic method used for auditing -all- admin commands.

    let player_name = if let Some(player) = players.get(client) {
        &*player.alias
    } else {
        warn!(
            "Failed to get player name for admin who used /disconnect_all_players - ignoring \
             command."
        );
        return Err(Content::localized("command-you-dont-exist"));
    };

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
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(a_skill_tree), Some(sp), entity_target) =
        parse_cmd_args!(args, String, u16, EntityTarget)
    {
        let skill_tree = parse_skill_tree(&a_skill_tree)?;
        let player = entity_target
            .map(|entity_target| get_entity_target(entity_target, server))
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
            Err(Content::Plain("Entity has no stats!".into()))
        }
    } else {
        Err(action.help_content())
    }
}

fn parse_skill_tree(skill_tree: &str) -> CmdResult<comp::skillset::SkillGroupKind> {
    use comp::{item::tool::ToolKind, skillset::SkillGroupKind};
    match skill_tree {
        "general" => Ok(SkillGroupKind::General),
        "sword" => Ok(SkillGroupKind::Weapon(ToolKind::Sword)),
        "axe" => Ok(SkillGroupKind::Weapon(ToolKind::Axe)),
        "hammer" => Ok(SkillGroupKind::Weapon(ToolKind::Hammer)),
        "bow" => Ok(SkillGroupKind::Weapon(ToolKind::Bow)),
        "staff" => Ok(SkillGroupKind::Weapon(ToolKind::Staff)),
        "sceptre" => Ok(SkillGroupKind::Weapon(ToolKind::Sceptre)),
        "mining" => Ok(SkillGroupKind::Weapon(ToolKind::Pick)),
        _ => Err(Content::localized_with_args(
            "command-invalid-skill-group",
            [("group", skill_tree)],
        )),
    }
}

fn reload_chunks_inner(server: &mut Server, pos: Vec3<f32>, radius: Option<i32>) -> usize {
    let mut removed = 0;

    if let Some(radius) = radius {
        let chunk_key = server.state.terrain().pos_key(pos.as_());

        for key_offset in Spiral2d::with_radius(radius) {
            let chunk_key = chunk_key + key_offset;

            #[cfg(feature = "persistent_world")]
            server
                .state
                .ecs()
                .try_fetch_mut::<crate::terrain_persistence::TerrainPersistence>()
                .map(|mut terrain_persistence| terrain_persistence.unload_chunk(chunk_key));
            if server.state.remove_chunk(chunk_key) {
                removed += 1;
            }
        }
    } else {
        #[cfg(feature = "persistent_world")]
        server
            .state
            .ecs()
            .try_fetch_mut::<crate::terrain_persistence::TerrainPersistence>()
            .map(|mut terrain_persistence| terrain_persistence.unload_all());
        removed = server.state.clear_terrain();
    }

    removed
}

fn handle_reload_chunks(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let radius = parse_cmd_args!(args, i32);

    let pos = position(server, target, "target")?.0;
    let removed = reload_chunks_inner(server, pos, radius.map(|radius| radius.clamp(0, 64)));

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized_with_args("command-reloaded-chunks", [(
                "reloaded",
                removed.to_string(),
            )]),
        ),
    );

    Ok(())
}

fn handle_remove_lights(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let opt_radius = parse_cmd_args!(args, f32);
    let player_pos = position(server, target, "target")?;
    let mut to_delete = vec![];

    let ecs = server.state.ecs();
    for (entity, pos, _, _, _) in (
        &ecs.entities(),
        &ecs.read_storage::<comp::Pos>(),
        &ecs.read_storage::<LightEmitter>(),
        !&ecs.read_storage::<WaypointArea>(),
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
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::Plain(format!("Removed {} lights!", size)),
        ),
    );
    Ok(())
}

fn get_entity_target(entity_target: EntityTarget, server: &Server) -> CmdResult<EcsEntity> {
    match entity_target {
        EntityTarget::Player(alias) => Ok(find_alias(server.state.ecs(), &alias, true)?.0),
        EntityTarget::RtsimNpc(id) => {
            let (npc_id, _) = server
                .state
                .ecs()
                .read_resource::<crate::rtsim::RtSim>()
                .state()
                .data()
                .npcs
                .iter()
                .find(|(_, npc)| npc.uid == id)
                .ok_or(Content::Plain(format!(
                    "Could not find rtsim npc with id {id}."
                )))?;
            server
                .state()
                .ecs()
                .read_resource::<common::uid::IdMaps>()
                .rtsim_entity(common::rtsim::RtSimEntity(npc_id))
                .ok_or(Content::Plain(format!("Npc with id {id} isn't loaded.")))
        },
        EntityTarget::Uid(uid) => server
            .state
            .ecs()
            .entity_from_uid(uid)
            .ok_or(Content::Plain(format!("{uid:?} not found."))),
    }
}

fn handle_sudo(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(entity_target), Some(cmd), cmd_args) =
        parse_cmd_args!(args, EntityTarget, String, ..Vec<String>)
    {
        if let Ok(action) = cmd.parse() {
            let entity = get_entity_target(entity_target, server)?;
            let client_uuid = uuid(server, client, "client")?;

            // If the entity target is a player check if client has authority to sudo it.
            {
                let players = server.state.ecs().read_storage::<comp::Player>();
                if let Some(player) = players.get(entity) {
                    let player_uuid = player.uuid();
                    drop(players);
                    verify_above_role(
                        server,
                        (client, client_uuid),
                        (entity, player_uuid),
                        Content::localized("command-sudo-higher-role"),
                    )?;
                } else if server.entity_admin_role(client) < Some(AdminRole::Admin) {
                    return Err(Content::localized(
                        "command-sudo-no-permission-for-non-players",
                    ));
                }
            }

            // TODO: consider making this into a tail call or loop (to avoid the potential
            // stack overflow, although it's less of a risk coming from only mods and
            // admins).
            do_command(server, client, entity, cmd_args, &action)
        } else {
            Err(Content::localized("command-unknown"))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_version(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized_with_args("command-version-current", [
                ("hash", (*common::util::GIT_HASH).to_owned()),
                ("date", (*common::util::GIT_DATE).to_owned()),
            ]),
        ),
    );
    Ok(())
}

fn handle_whitelist(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let now = Utc::now();

    if let (Some(whitelist_action), Some(username)) = parse_cmd_args!(args, String, String) {
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
                            Some(Content::localized_with_args("command-whitelist-added", [(
                                "username",
                                username.to_owned(),
                            )]))
                        }
                    });
            edit_setting_feedback(server, client, edit, || {
                Content::localized_with_args("command-whitelist-already-added", [(
                    "username", username,
                )])
            })
        } else if whitelist_action.eq_ignore_ascii_case("remove") {
            let client_uuid = uuid(server, client, "client")?;
            let client_role = real_role(server, client_uuid, "client")?;

            let uuid = find_username(server, &username)?;
            let mut err_key = "command-whitelist-unlisted";
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
                                    err_key = "command-whitelist-permission-denied";
                                    false
                                }
                            })
                            .map(|_| {
                                Content::localized_with_args("command-whitelist-removed", [(
                                    "username",
                                    username.to_owned(),
                                )])
                            })
                    });
            edit_setting_feedback(server, client, edit, || {
                Content::localized_with_args(err_key, [("username", username)])
            })
        } else {
            Err(action.help_content())
        }
    } else {
        Err(action.help_content())
    }
}

fn kick_player(
    server: &mut Server,
    (client, client_uuid): (EcsEntity, Uuid),
    (target_player, target_player_uuid): (EcsEntity, Uuid),
    reason: DisconnectReason,
) -> CmdResult<()> {
    verify_above_role(
        server,
        (client, client_uuid),
        (target_player, target_player_uuid),
        Content::localized("command-kick-higher-role"),
    )?;
    server.notify_client(target_player, ServerGeneral::Disconnect(reason));
    server
        .state
        .mut_resource::<EventBus<ClientDisconnectEvent>>()
        .emit_now(ClientDisconnectEvent(
            target_player,
            comp::DisconnectReason::Kicked,
        ));
    Ok(())
}

fn handle_kick(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(target_alias), reason_opt) = parse_cmd_args!(args, String, String) {
        let client_uuid = uuid(server, client, "client")?;
        let reason = reason_opt.unwrap_or_default();
        let ecs = server.state.ecs();
        let target_player = find_alias(ecs, &target_alias, true)?;

        kick_player(
            server,
            (client, client_uuid),
            target_player,
            DisconnectReason::Kicked(reason.clone()),
        )?;
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::Plain(format!(
                    "Kicked {} from the server with reason: {}",
                    target_alias, reason
                )),
            ),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn make_ban_info(server: &mut Server, client: EcsEntity, client_uuid: Uuid) -> CmdResult<BanInfo> {
    let client_username = uuid_to_username(server, client, client_uuid)?;
    let client_role = real_role(server, client_uuid, "client")?;
    let ban_info = BanInfo {
        performed_by: client_uuid,
        performed_by_username: client_username,
        performed_by_role: client_role.into(),
    };
    Ok(ban_info)
}

fn ban_end_date(
    now: chrono::DateTime<Utc>,
    parse_duration: Option<HumanDuration>,
) -> CmdResult<Option<chrono::DateTime<Utc>>> {
    let end_date = parse_duration
        .map(|duration| chrono::Duration::from_std(duration.into()))
        .transpose()
        .map_err(|err| {
            Content::localized_with_args(
                "command-parse-duration-error",
                [("error", format!("{err:?}"))]
            )
        })?
        // On overflow (someone adding some ridiculous time span), just make the ban infinite.
        // (end date of None is an infinite ban)
        .and_then(|duration| now.checked_add_signed(duration));
    Ok(end_date)
}

fn handle_ban(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(username), overwrite, parse_duration, reason_opt) =
        parse_cmd_args!(args, String, bool, HumanDuration, String)
    {
        let reason = reason_opt.unwrap_or_default();
        let overwrite = overwrite.unwrap_or(false);

        let client_uuid = uuid(server, client, "client")?;
        let ban_info = make_ban_info(server, client, client_uuid)?;

        let player_uuid = find_username(server, &username)?;

        let now = Utc::now();
        let end_date = ban_end_date(now, parse_duration)?;

        let result = server.editable_settings_mut().banlist.ban_operation(
            server.data_dir().as_ref(),
            now,
            player_uuid,
            username.clone(),
            BanOperation::Ban {
                reason: reason.clone(),
                info: ban_info,
                end_date,
            },
            overwrite,
        );
        let (result, ban_info) = match result {
            Ok(info) => (Ok(()), info),
            Err(err) => (Err(err), None),
        };

        edit_banlist_feedback(
            server,
            client,
            result,
            || {
                Content::localized_with_args("command-ban-added", [
                    ("player", username.clone()),
                    ("reason", reason),
                ])
            },
            || {
                Content::localized_with_args("command-ban-already-added", [(
                    "player",
                    username.clone(),
                )])
            },
        )?;
        // If the player is online kick them (this may fail if the player is a hardcoded
        // admin; we don't care about that case because hardcoded admins can log on even
        // if they're on the ban list).
        let ecs = server.state.ecs();
        if let Ok(target_player) = find_uuid(ecs, player_uuid) {
            let _ = kick_player(
                server,
                (client, client_uuid),
                (target_player, player_uuid),
                ban_info.map_or(DisconnectReason::Shutdown, DisconnectReason::Banned),
            );
        }
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_aura(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let target_uid = uid(server, target, "target")?;

    let (Some(aura_radius), aura_duration, new_entity, aura_target, Some(aura_kind_variant), spec) =
        parse_cmd_args!(args, f32, f32, bool, GroupTarget, AuraKindVariant, ..Vec<String>)
    else {
        return Err(action.help_content());
    };
    let new_entity = new_entity.unwrap_or(false);
    let aura_kind = match aura_kind_variant {
        AuraKindVariant::Buff => {
            let (Some(buff), strength, duration, misc_data_spec) =
                parse_cmd_args!(spec, String, f32, f64, String)
            else {
                return Err(Content::localized("command-aura-invalid-buff-parameters"));
            };
            let buffkind = parse_buffkind(&buff).ok_or_else(|| {
                Content::localized_with_args("command-buff-unknown", [("buff", buff.clone())])
            })?;
            let buffdata = build_buff(
                buffkind,
                strength.unwrap_or(1.0),
                duration.unwrap_or(10.0),
                (!buffkind.is_simple())
                    .then(|| {
                        misc_data_spec.ok_or_else(|| {
                            Content::localized_with_args("command-buff-data", [(
                                "buff",
                                buff.clone(),
                            )])
                        })
                    })
                    .transpose()?,
            )?;

            AuraKind::Buff {
                kind: buffkind,
                data: buffdata,
                category: BuffCategory::Natural,
                source: if new_entity {
                    BuffSource::World
                } else {
                    BuffSource::Character { by: target_uid }
                },
            }
        },
        AuraKindVariant::FriendlyFire => AuraKind::FriendlyFire,
        AuraKindVariant::ForcePvP => AuraKind::ForcePvP,
    };
    let aura_target = server
        .state
        .read_component_copied::<Uid>(target)
        .map(|uid| match aura_target {
            Some(GroupTarget::InGroup) => AuraTarget::GroupOf(uid),
            Some(GroupTarget::OutOfGroup) => AuraTarget::NotGroupOf(uid),
            Some(GroupTarget::All) | None => AuraTarget::All,
        })
        .unwrap_or(AuraTarget::All);

    let time = Time(server.state.get_time());
    let aura = Aura::new(
        aura_kind,
        aura_radius,
        aura_duration.map(|duration| Secs(duration as f64)),
        aura_target,
        time,
    );

    if new_entity {
        let pos = position(server, target, "target")?;
        server
            .state
            .create_empty(pos)
            .with(comp::Auras::new(vec![aura]))
            .maybe_with(aura_duration.map(|duration| comp::Object::DeleteAfter {
                spawned_at: time,
                timeout: Duration::from_secs_f32(duration),
            }))
            .build();
    } else {
        let mut auras = server.state.ecs().write_storage::<comp::Auras>();
        if let Some(mut auras) = auras.get_mut(target) {
            auras.insert(aura);
        }
    }

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized(if new_entity {
                "command-aura-spawn-new-entity"
            } else {
                "command-aura-spawn"
            }),
        ),
    );

    Ok(())
}

fn handle_ban_ip(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(username), overwrite, parse_duration, reason_opt) =
        parse_cmd_args!(args, String, bool, HumanDuration, String)
    {
        let reason = reason_opt.unwrap_or_default();
        let overwrite = overwrite.unwrap_or(false);

        let client_uuid = uuid(server, client, "client")?;
        let ban_info = make_ban_info(server, client, client_uuid)?;

        let player_uuid = find_username(server, &username)?;
        let player_entity = find_uuid(server.state.ecs(), player_uuid).map_err(|err| {
            Content::localized_with_args("command-ip-ban-require-online", [("error", err)])
        })?;
        let player_ip_addr =
            NormalizedIpAddr::from(socket_addr(server, player_entity, &username)?.ip());

        let now = Utc::now();
        let end_date = ban_end_date(now, parse_duration)?;

        let result = server.editable_settings_mut().banlist.ban_operation(
            server.data_dir().as_ref(),
            now,
            player_uuid,
            username.clone(),
            BanOperation::BanIp {
                reason: reason.clone(),
                info: ban_info,
                end_date,
                ip: player_ip_addr,
            },
            overwrite,
        );
        let (result, ban_info) = match result {
            Ok(info) => (Ok(()), info),
            Err(err) => (Err(err), None),
        };

        edit_banlist_feedback(
            server,
            client,
            result,
            || {
                Content::localized_with_args("command-ban-ip-added", [
                    ("player", username.clone()),
                    ("reason", reason),
                ])
            },
            || {
                Content::localized_with_args("command-ban-already-added", [(
                    "player",
                    username.clone(),
                )])
            },
        )?;

        // Kick all online players with this IP address them (this may fail if the
        // player is a hardcoded admin; we don't care about that case because
        // hardcoded admins can log on even if they're on the ban list).
        let ecs = server.state.ecs();
        let players_to_kick = (
            &ecs.entities(),
            &ecs.read_storage::<Client>(),
            &ecs.read_storage::<comp::Player>(),
        )
            .join()
            .filter(|(_, client, _)| {
                client
                    .current_ip_addrs
                    .iter()
                    .any(|socket_addr| NormalizedIpAddr::from(socket_addr.ip()) == player_ip_addr)
            })
            .map(|(entity, _, player)| (entity, player.uuid()))
            .collect::<Vec<_>>();
        for (player_entity, player_uuid) in players_to_kick {
            let _ = kick_player(
                server,
                (client, client_uuid),
                (player_entity, player_uuid),
                ban_info
                    .clone()
                    .map_or(DisconnectReason::Shutdown, DisconnectReason::Banned),
            );
        }
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_battlemode(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(argument) = parse_cmd_args!(args, String) {
        let battle_mode = match argument.as_str() {
            "pvp" => BattleMode::PvP,
            "pve" => BattleMode::PvE,
            _ => return Err(Content::localized("command-battlemode-available-modes")),
        };

        server.set_battle_mode_for(client, battle_mode);
    } else {
        server.get_battle_mode_for(client);
    }

    Ok(())
}

fn handle_battlemode_force(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let ecs = server.state.ecs();
    let settings = ecs.read_resource::<Settings>();

    if !settings.gameplay.battle_mode.allow_choosing() {
        return Err(Content::localized("command-disabled-by-settings"));
    }

    let mode = parse_cmd_args!(args, String).ok_or_else(|| action.help_content())?;
    let mode = match mode.as_str() {
        "pvp" => BattleMode::PvP,
        "pve" => BattleMode::PvE,
        _ => return Err(Content::localized("command-battlemode-available-modes")),
    };

    let mut players = ecs.write_storage::<comp::Player>();
    let mut player_info = players.get_mut(target).ok_or(Content::Plain(
        "Cannot get player component for target".to_string(),
    ))?;
    player_info.battle_mode = mode;

    server.notify_client(
        client,
        ServerGeneral::server_msg(
            ChatType::CommandInfo,
            Content::localized_with_args("command-battlemode-updated", [(
                "battlemode",
                format!("{mode:?}"),
            )]),
        ),
    );
    Ok(())
}

fn handle_unban(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(username) = parse_cmd_args!(args, String) {
        let player_uuid = find_username(server, &username)?;

        let client_uuid = uuid(server, client, "client")?;
        let ban_info = make_ban_info(server, client, client_uuid)?;

        let now = Utc::now();

        let unban = BanOperation::Unban { info: ban_info };

        let result = server.editable_settings_mut().banlist.ban_operation(
            server.data_dir().as_ref(),
            now,
            player_uuid,
            username.clone(),
            unban,
            false,
        );

        edit_banlist_feedback(
            server,
            client,
            result.map(|_| ()),
            // TODO: it would be useful to indicate here whether an IP ban was also removed but we
            // don't have that info.
            || {
                Content::localized_with_args("command-unban-successful", [(
                    "player",
                    username.clone(),
                )])
            },
            || {
                Content::localized_with_args("command-unban-already-unbanned", [(
                    "player",
                    username.clone(),
                )])
            },
        )
    } else {
        Err(action.help_content())
    }
}

fn handle_unban_ip(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(username) = parse_cmd_args!(args, String) {
        let player_uuid = find_username(server, &username)?;

        let client_uuid = uuid(server, client, "client")?;
        let ban_info = make_ban_info(server, client, client_uuid)?;

        let now = Utc::now();

        let unban = BanOperation::UnbanIp {
            info: ban_info,
            uuid: player_uuid,
        };

        let result = server.editable_settings_mut().banlist.ban_operation(
            server.data_dir().as_ref(),
            now,
            player_uuid,
            username.clone(),
            unban,
            false,
        );

        edit_banlist_feedback(
            server,
            client,
            result.map(|_| ()),
            || {
                Content::localized_with_args("command-unban-ip-successful", [(
                    "player",
                    username.clone(),
                )])
            },
            || {
                Content::localized_with_args("command-unban-already-unbanned", [(
                    "player",
                    username.clone(),
                )])
            },
        )
    } else {
        Err(action.help_content())
    }
}

fn handle_server_physics(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(username), enabled_opt, reason) = parse_cmd_args!(args, String, bool, String) {
        let uuid = find_username(server, &username)?;
        let server_force = enabled_opt.unwrap_or(true);
        let data_dir = server.data_dir();

        let result = server
            .editable_settings_mut()
            .server_physics_force_list
            .edit(data_dir.as_ref(), |list| {
                if server_force {
                    let Some(by) = server
                        .state()
                        .ecs()
                        .read_storage::<comp::Player>()
                        .get(client)
                        .map(|player| (player.uuid(), player.alias.clone()))
                    else {
                        return Some(Some(Content::localized("command-you-dont-exist")));
                    };
                    list.insert(uuid, ServerPhysicsForceRecord {
                        by: Some(by),
                        reason,
                    });
                    Some(None)
                } else {
                    list.remove(&uuid);
                    Some(None)
                }
            });

        if let Some((Some(error), _)) = result {
            return Err(error);
        }

        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::Plain(format!(
                    "Updated physics settings for {} ({}): {:?}",
                    username, uuid, server_force
                )),
            ),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_buff(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let (Some(buff), strength, duration, misc_data_spec) =
        parse_cmd_args!(args, String, f32, f64, String)
    else {
        return Err(action.help_content());
    };

    let strength = strength.unwrap_or(0.01);

    match buff.as_str() {
        "all" => {
            let duration = duration.unwrap_or(5.0);
            let buffdata = BuffData::new(strength, Some(Secs(duration)));

            // apply every(*) non-complex buff
            //
            // (*) BUFF_PACK contains all buffs except
            // invulnerability
            BUFF_PACK
                .iter()
                .filter_map(|kind_key| parse_buffkind(kind_key))
                .filter(|buffkind| buffkind.is_simple())
                .for_each(|buffkind| cast_buff(buffkind, buffdata, server, target));
        },
        "clear" => {
            if let Some(mut buffs) = server
                .state
                .ecs()
                .write_storage::<comp::Buffs>()
                .get_mut(target)
            {
                buffs.buffs.clear();
                buffs.kinds.clear();
            }
        },
        _ => {
            let buffkind = parse_buffkind(&buff).ok_or_else(|| {
                Content::localized_with_args("command-buff-unknown", [("buff", buff.clone())])
            })?;
            let buffdata = build_buff(
                buffkind,
                strength,
                duration.unwrap_or(match buffkind {
                    BuffKind::ComboGeneration => 1.0,
                    _ => 10.0,
                }),
                (!buffkind.is_simple())
                    .then(|| {
                        misc_data_spec.ok_or_else(|| {
                            Content::localized_with_args("command-buff-data", [(
                                "buff",
                                buff.clone(),
                            )])
                        })
                    })
                    .transpose()?,
            )?;

            cast_buff(buffkind, buffdata, server, target);
        },
    }

    Ok(())
}

fn build_buff(
    buff_kind: BuffKind,
    strength: f32,
    duration: f64,
    spec: Option<String>,
) -> CmdResult<BuffData> {
    if buff_kind.is_simple() {
        Ok(BuffData::new(strength, Some(Secs(duration))))
    } else {
        let spec = spec.expect("spec must be passed to build_buff if buff_kind is not simple");

        // Explicit match to remember that this function exists
        let misc_data = match buff_kind {
            BuffKind::Polymorphed => {
                let Ok(npc::NpcBody(_id, mut body)) = spec.parse() else {
                    return Err(Content::localized_with_args("command-buff-body-unknown", [
                        ("spec", spec.clone()),
                    ]));
                };
                MiscBuffData::Body(body())
            },
            BuffKind::Regeneration
            | BuffKind::Saturation
            | BuffKind::Potion
            | BuffKind::Agility
            | BuffKind::RestingHeal
            | BuffKind::Frenzied
            | BuffKind::EnergyRegen
            | BuffKind::ComboGeneration
            | BuffKind::IncreaseMaxEnergy
            | BuffKind::IncreaseMaxHealth
            | BuffKind::Invulnerability
            | BuffKind::ProtectingWard
            | BuffKind::Hastened
            | BuffKind::Fortitude
            | BuffKind::Reckless
            | BuffKind::Flame
            | BuffKind::Frigid
            | BuffKind::Lifesteal
            | BuffKind::ImminentCritical
            | BuffKind::Fury
            | BuffKind::Sunderer
            | BuffKind::Defiance
            | BuffKind::Bloodfeast
            | BuffKind::Berserk
            | BuffKind::Bleeding
            | BuffKind::Cursed
            | BuffKind::Burning
            | BuffKind::Crippled
            | BuffKind::Frozen
            | BuffKind::Wet
            | BuffKind::Ensnared
            | BuffKind::Poisoned
            | BuffKind::Parried
            | BuffKind::PotionSickness
            | BuffKind::Heatstroke
            | BuffKind::ScornfulTaunt
            | BuffKind::Rooted
            | BuffKind::Winded
            | BuffKind::Concussion
            | BuffKind::Staggered
            | BuffKind::Tenacity
            | BuffKind::Resilience => {
                if buff_kind.is_simple() {
                    unreachable!("is_simple() above")
                } else {
                    panic!("Buff Kind {buff_kind:?} is complex but has no defined spec parser")
                }
            },
        };

        Ok(BuffData::new(strength, Some(Secs(duration))).with_misc_data(misc_data))
    }
}

fn cast_buff(buffkind: BuffKind, data: BuffData, server: &mut Server, target: EcsEntity) {
    let ecs = &server.state.ecs();
    let mut buffs_all = ecs.write_storage::<comp::Buffs>();
    let stats = ecs.read_storage::<comp::Stats>();
    let masses = ecs.read_storage::<comp::Mass>();
    let time = ecs.read_resource::<Time>();
    if let Some(mut buffs) = buffs_all.get_mut(target) {
        let dest_info = DestInfo {
            stats: stats.get(target),
            mass: masses.get(target),
        };
        buffs.insert(
            Buff::new(
                buffkind,
                data,
                vec![],
                BuffSource::Command,
                *time,
                dest_info,
                None,
            ),
            *time,
        );
    }
}

fn parse_buffkind(buff: &str) -> Option<BuffKind> { BUFF_PARSER.get(buff).copied() }

fn handle_skill_preset(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(preset) = parse_cmd_args!(args, String) {
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
            Err(Content::Plain("Player has no stats!".into()))
        }
    } else {
        Err(action.help_content())
    }
}

fn clear_skillset(skill_set: &mut comp::SkillSet) { *skill_set = comp::SkillSet::default(); }

fn set_skills(skill_set: &mut comp::SkillSet, preset: &str) -> CmdResult<()> {
    let presets = match common::cmd::SkillPresetManifest::load(PRESET_MANIFEST_PATH) {
        Ok(presets) => presets.read().0.clone(),
        Err(err) => {
            warn!("Error in preset: {}", err);
            return Err(Content::localized("command-skillpreset-load-error"));
        },
    };
    if let Some(preset) = presets.get(preset) {
        for (skill, level) in preset {
            let group = if let Some(group) = skill.skill_group_kind() {
                group
            } else {
                warn!("Skill in preset doesn't exist in any group");
                return Err(Content::localized("command-skillpreset-broken"));
            };
            for _ in 0..*level {
                let cost = skill_set.skill_cost(*skill);
                skill_set.add_skill_points(group, cost);
                match skill_set.unlock_skill(*skill) {
                    Ok(_) | Err(comp::skillset::SkillUnlockError::SkillAlreadyUnlocked) => Ok(()),
                    Err(err) => Err(Content::Plain(format!("{:?}", err))),
                }?;
            }
        }
        Ok(())
    } else {
        Err(Content::localized_with_args(
            "command-skillpreset-missing",
            [("preset", preset)],
        ))
    }
}

fn handle_location(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(name) = parse_cmd_args!(args, String) {
        let loc = server.state.ecs().read_resource::<Locations>().get(&name)?;
        server.state.position_mut(target, true, |target_pos| {
            target_pos.0 = loc;
        })
    } else {
        let locations = server.state.ecs().read_resource::<Locations>();
        let mut locations = locations.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        locations.sort_unstable();
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                if locations.is_empty() {
                    Content::localized("command-locations-empty")
                } else {
                    Content::localized_with_args("command-locations-list", [(
                        "locations",
                        locations.join(", "),
                    )])
                },
            ),
        );
        Ok(())
    }
}

fn handle_create_location(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(name) = parse_cmd_args!(args, String) {
        let target_pos = position(server, target, "target")?;

        server
            .state
            .ecs_mut()
            .write_resource::<Locations>()
            .insert(name.clone(), target_pos.0)?;
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized_with_args("command-location-created", [("location", name)]),
            ),
        );

        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_delete_location(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(name) = parse_cmd_args!(args, String) {
        server
            .state
            .ecs_mut()
            .write_resource::<Locations>()
            .remove(&name)?;
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized_with_args("command-location-deleted", [("location", name)]),
            ),
        );

        Ok(())
    } else {
        Err(action.help_content())
    }
}

#[cfg(not(feature = "worldgen"))]
fn handle_weather_zone(
    _server: &mut Server,
    _client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    Err(Content::Plain(
        "Unsupported without worldgen enabled".into(),
    ))
}

#[cfg(feature = "worldgen")]
fn handle_weather_zone(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(name), radius, time) = parse_cmd_args!(args, String, f32, f32) {
        let radius = radius.map(|r| r / weather::CELL_SIZE as f32).unwrap_or(1.0);
        let time = time.unwrap_or(100.0);

        let mut add_zone = |weather: weather::Weather| {
            if let Ok(pos) = position(server, client, "player") {
                let pos = pos.0.xy() / weather::CELL_SIZE as f32;
                if let Some(weather_job) = server
                    .state
                    .ecs_mut()
                    .write_resource::<Option<WeatherJob>>()
                    .as_mut()
                {
                    weather_job.queue_zone(weather, pos, radius, time);
                }
            }
        };
        match name.as_str() {
            "clear" => {
                add_zone(weather::Weather {
                    cloud: 0.0,
                    rain: 0.0,
                    wind: Vec2::zero(),
                });
                Ok(())
            },
            "cloudy" => {
                add_zone(weather::Weather {
                    cloud: 0.4,
                    rain: 0.0,
                    wind: Vec2::zero(),
                });
                Ok(())
            },
            "rain" => {
                add_zone(weather::Weather {
                    cloud: 0.1,
                    rain: 0.15,
                    wind: Vec2::new(1.0, -1.0),
                });
                Ok(())
            },
            "wind" => {
                add_zone(weather::Weather {
                    cloud: 0.0,
                    rain: 0.0,
                    wind: Vec2::new(10.0, 10.0),
                });
                Ok(())
            },
            "storm" => {
                add_zone(weather::Weather {
                    cloud: 0.3,
                    rain: 0.3,
                    wind: Vec2::new(15.0, 20.0),
                });
                Ok(())
            },
            _ => Err(Content::localized("command-weather-valid-values")),
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_lightning(
    server: &mut Server,
    client: EcsEntity,
    _target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let pos = position(server, client, "player")?.0;
    server
        .state
        .ecs()
        .read_resource::<EventBus<Outcome>>()
        .emit_now(Outcome::Lightning { pos });
    Ok(())
}

fn assign_body(server: &mut Server, target: EcsEntity, body: comp::Body) -> CmdResult<()> {
    insert_or_replace_component(server, target, body, "body")?;
    insert_or_replace_component(server, target, body.mass(), "mass")?;
    insert_or_replace_component(server, target, body.density(), "density")?;
    insert_or_replace_component(server, target, body.collider(), "collider")?;

    if let Some(mut stat) = server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(target)
    {
        stat.original_body = body;
    }

    Ok(())
}

fn handle_body(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(npc::NpcBody(_id, mut body)) = parse_cmd_args!(args, npc::NpcBody) {
        let body = body();

        assign_body(server, target, body)
    } else {
        Err(action.help_content())
    }
}

fn handle_scale(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let (Some(scale), reset_mass) = parse_cmd_args!(args, f32, bool) {
        let scale = scale.clamped(0.025, 1000.0);
        insert_or_replace_component(server, target, comp::Scale(scale), "target")?;
        if reset_mass.unwrap_or(true) {
            let mass = server.state.ecs()
                .read_storage::<comp::Body>()
                .get(target)
                // Mass is derived from volume, which changes with the third power of scale
                .map(|body| body.mass().0 * scale.powi(3));
            if let Some(mass) = mass {
                insert_or_replace_component(server, target, comp::Mass(mass), "target")?;
            }
        }
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized_with_args("command-scale-set", [(
                    "scale",
                    format!("{scale:.1}"),
                )]),
            ),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

// /repair_equipment <false/true>
fn handle_repair_equipment(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let repair_inventory = parse_cmd_args!(args, bool).unwrap_or(false);
    let ecs = server.state.ecs();
    if let Some(mut inventory) = ecs.write_storage::<comp::Inventory>().get_mut(target) {
        let ability_map = ecs.read_resource::<AbilityMap>();
        let msm = ecs.read_resource::<MaterialStatManifest>();
        let slots = inventory
            .equipped_items_with_slot()
            .filter(|(_, item)| item.has_durability())
            .map(|(slot, _)| Slot::Equip(slot))
            .chain(
                repair_inventory
                    .then(|| {
                        inventory
                            .slots_with_id()
                            .filter(|(_, item)| {
                                item.as_ref().is_some_and(|item| item.has_durability())
                            })
                            .map(|(slot, _)| Slot::Inventory(slot))
                    })
                    .into_iter()
                    .flatten(),
            )
            .collect::<Vec<Slot>>();

        for slot in slots {
            inventory.repair_item_at_slot(slot, &ability_map, &msm);
        }

        let key = if repair_inventory {
            "command-repaired-inventory_items"
        } else {
            "command-repaired-items"
        };
        server.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, Content::localized(key)),
        );
        Ok(())
    } else {
        Err(action.help_content())
    }
}

fn handle_tether(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    enum Either<A, B> {
        Left(A),
        Right(B),
    }

    impl<A: FromStr, B: FromStr> FromStr for Either<A, B> {
        type Err = B::Err;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            A::from_str(s)
                .map(Either::Left)
                .or_else(|_| B::from_str(s).map(Either::Right))
        }
    }
    if let (Some(entity_target), length) = parse_cmd_args!(args, EntityTarget, Either<f32, bool>) {
        let entity_target = get_entity_target(entity_target, server)?;

        let tether_leader = server.state.ecs().uid_from_entity(target);
        let tether_follower = server.state.ecs().uid_from_entity(entity_target);

        if let (Some(leader), Some(follower)) = (tether_leader, tether_follower) {
            let base_len = server
                .state
                .read_component_cloned::<comp::Body>(target)
                .map(|b| b.dimensions().y * 1.5 + 1.0)
                .unwrap_or(6.0);
            let tether_length = match length {
                Some(Either::Left(l)) => l.max(0.0) + base_len,
                Some(Either::Right(true)) => {
                    let leader_pos = position(server, target, "leader")?;
                    let follower_pos = position(server, entity_target, "follower")?;

                    leader_pos.0.distance(follower_pos.0) + base_len
                },
                _ => base_len,
            };
            server
                .state
                .link(Tethered {
                    leader,
                    follower,
                    tether_length,
                })
                .map_err(|_| Content::Plain("Failed to tether entities".into()))
        } else {
            Err(Content::Plain("Tether members don't have Uids.".into()))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_destroy_tethers(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let mut destroyed = false;
    destroyed |= server
        .state
        .ecs()
        .write_storage::<Is<common::tether::Leader>>()
        .remove(target)
        .is_some();
    destroyed |= server
        .state
        .ecs()
        .write_storage::<Is<common::tether::Follower>>()
        .remove(target)
        .is_some();
    if destroyed {
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized("command-destroyed-tethers"),
            ),
        );
        Ok(())
    } else {
        Err(Content::localized("command-destroyed-no-tethers"))
    }
}

fn handle_mount(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    if let Some(entity_target) = parse_cmd_args!(args, EntityTarget) {
        let entity_target = get_entity_target(entity_target, server)?;

        let rider = server.state.ecs().uid_from_entity(target);
        let mount = server.state.ecs().uid_from_entity(entity_target);

        if let (Some(rider), Some(mount)) = (rider, mount) {
            server
                .state
                .link(common::mounting::Mounting { mount, rider })
                .map_err(|_| Content::Plain("Failed to mount entities".into()))
        } else {
            Err(Content::Plain(
                "Mount and/or rider doesn't have an Uid component.".into(),
            ))
        }
    } else {
        Err(action.help_content())
    }
}

fn handle_dismount(
    server: &mut Server,
    client: EcsEntity,
    target: EcsEntity,
    _args: Vec<String>,
    _action: &ServerChatCommand,
) -> CmdResult<()> {
    let mut destroyed = false;
    destroyed |= server
        .state
        .ecs()
        .write_storage::<Is<common::mounting::Rider>>()
        .remove(target)
        .is_some();
    destroyed |= server
        .state
        .ecs()
        .write_storage::<Is<common::mounting::VolumeRider>>()
        .remove(target)
        .is_some();
    destroyed |= server
        .state
        .ecs()
        .write_storage::<Is<common::mounting::Mount>>()
        .remove(target)
        .is_some();
    destroyed |= server
        .state
        .ecs()
        .write_storage::<common::mounting::VolumeRiders>()
        .get_mut(target)
        .is_some_and(|volume_riders| volume_riders.clear());

    if destroyed {
        server.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized("command-dismounted"),
            ),
        );
        Ok(())
    } else {
        Err(Content::localized("command-no-dismount"))
    }
}

#[cfg(feature = "worldgen")]
fn handle_spot(
    server: &mut Server,
    _client: EcsEntity,
    target: EcsEntity,
    args: Vec<String>,
    action: &ServerChatCommand,
) -> CmdResult<()> {
    let Some(target_spot) = parse_cmd_args!(args, String) else {
        return Err(action.help_content());
    };

    let maybe_spot_kind = SPOT_PARSER.get(&target_spot);

    let target_pos = server
        .state
        .read_component_copied::<comp::Pos>(target)
        .ok_or(Content::localized_with_args(
            "command-position-unavailable",
            [("target", "target")],
        ))?;
    let target_chunk = target_pos.0.xy().wpos_to_cpos().as_();

    let world = server.state.ecs().read_resource::<Arc<world::World>>();
    let spot_chunk = Spiral2d::new()
        .map(|o| target_chunk + o)
        .filter(|chunk| world.sim().get(*chunk).is_some())
        .take(world.sim().map_size_lg().chunks_len())
        .find(|chunk| {
            world.sim().get(*chunk).is_some_and(|chunk| {
                if let Some(spot) = &chunk.spot {
                    match spot {
                        Spot::RonFile(spot) => spot.base_structures == target_spot,
                        spot_kind => Some(spot_kind) == maybe_spot_kind,
                    }
                } else {
                    false
                }
            })
        });

    if let Some(spot_chunk) = spot_chunk {
        let pos = spot_chunk.cpos_to_wpos_center();
        // NOTE: teleport somewhere higher to avoid spawning inside the spot
        //
        // Get your glider ready!
        let uplift = 100.0;
        let pos = (pos.as_() + 0.5).with_z(world.sim().get_surface_alt_approx(pos) + uplift);
        drop(world);
        server.state.position_mut(target, true, |target_pos| {
            *target_pos = comp::Pos(pos);
        })?;
        Ok(())
    } else {
        Err(Content::localized("command-spot-spot_not_found"))
    }
}

#[cfg(not(feature = "worldgen"))]
fn handle_spot(
    _: &mut Server,
    _: EcsEntity,
    _: EcsEntity,
    _: Vec<String>,
    _: &ServerChatCommand,
) -> CmdResult<()> {
    Err(Content::localized("command-spot-world_feature"))
}
