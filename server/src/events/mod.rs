use crate::{
    events::interaction::handle_tame_pet, persistence::PersistedComponents, state_ext::StateExt,
    Server,
};
use common::event::{EventBus, ServerEvent, ServerEventDiscriminants};
use common_base::span;
use entity_creation::{
    handle_beam, handle_create_npc, handle_create_ship, handle_create_waypoint,
    handle_initialize_character, handle_initialize_spectator, handle_loaded_character_data,
    handle_shockwave, handle_shoot,
};
use entity_manipulation::{
    handle_aura, handle_bonk, handle_buff, handle_change_ability, handle_combo_change,
    handle_delete, handle_destroy, handle_energy_change, handle_entity_attacked_hook,
    handle_explosion, handle_health_change, handle_knockback, handle_land_on_ground,
    handle_make_admin, handle_parry_hook, handle_poise, handle_respawn, handle_stance_change,
    handle_teleport_to, handle_update_map_marker,
};
use group_manip::handle_group;
use information::handle_site_info;
use interaction::{
    handle_create_sprite, handle_lantern, handle_mine_block, handle_mount, handle_npc_interaction,
    handle_sound, handle_unmount,
};
use inventory_manip::handle_inventory;
use invite::{handle_invite, handle_invite_response};
use player::{handle_client_disconnect, handle_exit_ingame, handle_possess};
use specs::{Builder, Entity as EcsEntity, WorldExt};
use trade::handle_process_trade_action;

use crate::events::player::handle_character_delete;
pub use group_manip::update_map_markers;
pub(crate) use trade::cancel_trades_for;

mod entity_creation;
mod entity_manipulation;
mod group_manip;
mod information;
mod interaction;
mod inventory_manip;
mod invite;
mod player;
mod trade;

pub enum Event {
    ClientConnected {
        entity: EcsEntity,
    },
    ClientDisconnected {
        entity: EcsEntity,
    },
    Chat {
        entity: Option<EcsEntity>,
        msg: String,
    },
}

impl Server {
    pub fn handle_events(&mut self) -> Vec<Event> {
        span!(_guard, "handle_events", "Server::handle_events");
        let mut frontend_events = Vec::new();

        let mut commands = Vec::new();
        let mut chat_messages = Vec::new();

        let events = self
            .state
            .ecs()
            .read_resource::<EventBus<ServerEvent>>()
            .recv_all();

        use strum::VariantNames;
        let mut event_counts = vec![0u32; ServerEventDiscriminants::VARIANTS.len()];

        for event in events {
            // Count events by variant for metrics
            event_counts[ServerEventDiscriminants::from(&event) as usize] += 1;

            match event {
                ServerEvent::Explosion {
                    pos,
                    explosion,
                    owner,
                } => handle_explosion(self, pos, explosion, owner),
                ServerEvent::Bonk { pos, owner, target } => handle_bonk(self, pos, owner, target),
                ServerEvent::Shoot {
                    entity,
                    pos,
                    dir,
                    body,
                    light,
                    projectile,
                    speed,
                    object,
                } => handle_shoot(
                    self, entity, pos, dir, body, light, projectile, speed, object,
                ),
                ServerEvent::Shockwave {
                    properties,
                    pos,
                    ori,
                } => handle_shockwave(self, properties, pos, ori),
                ServerEvent::BeamSegment {
                    properties,
                    pos,
                    ori,
                } => handle_beam(self, properties, pos, ori),
                ServerEvent::Knockback { entity, impulse } => {
                    handle_knockback(self, entity, impulse)
                },
                ServerEvent::HealthChange { entity, change } => {
                    handle_health_change(self, entity, change)
                },
                ServerEvent::PoiseChange { entity, change } => handle_poise(self, entity, change),
                ServerEvent::Delete(entity) => handle_delete(self, entity),
                ServerEvent::Destroy { entity, cause } => handle_destroy(self, entity, cause),
                ServerEvent::InventoryManip(entity, manip) => handle_inventory(self, entity, manip),
                ServerEvent::GroupManip(entity, manip) => handle_group(self, entity, manip),
                ServerEvent::Respawn(entity) => handle_respawn(self, entity),
                ServerEvent::LandOnGround { entity, vel } => {
                    handle_land_on_ground(self, entity, vel)
                },
                ServerEvent::EnableLantern(entity) => handle_lantern(self, entity, true),
                ServerEvent::DisableLantern(entity) => handle_lantern(self, entity, false),
                ServerEvent::NpcInteract(interactor, target) => {
                    handle_npc_interaction(self, interactor, target)
                },
                ServerEvent::InitiateInvite(interactor, target, kind) => {
                    handle_invite(self, interactor, target, kind)
                },
                ServerEvent::InviteResponse(entity, response) => {
                    handle_invite_response(self, entity, response)
                },
                ServerEvent::ProcessTradeAction(entity, trade_id, action) => {
                    handle_process_trade_action(self, entity, trade_id, action);
                },
                ServerEvent::Mount(mounter, mountee) => handle_mount(self, mounter, mountee),
                ServerEvent::Unmount(mounter) => handle_unmount(self, mounter),
                ServerEvent::Possess(possessor_uid, possesse_uid) => {
                    handle_possess(self, possessor_uid, possesse_uid)
                },
                ServerEvent::InitCharacterData {
                    entity,
                    character_id,
                    requested_view_distances,
                } => handle_initialize_character(
                    self,
                    entity,
                    character_id,
                    requested_view_distances,
                ),
                ServerEvent::InitSpectator(entity, requested_view_distances) => {
                    handle_initialize_spectator(self, entity, requested_view_distances)
                },
                ServerEvent::DeleteCharacter {
                    entity,
                    requesting_player_uuid,
                    character_id,
                } => handle_character_delete(self, entity, requesting_player_uuid, character_id),
                ServerEvent::UpdateCharacterData {
                    entity,
                    components,
                    metadata,
                } => {
                    let (
                        body,
                        stats,
                        skill_set,
                        inventory,
                        waypoint,
                        pets,
                        active_abilities,
                        map_marker,
                    ) = components;
                    let components = PersistedComponents {
                        body,
                        stats,
                        skill_set,
                        inventory,
                        waypoint,
                        pets,
                        active_abilities,
                        map_marker,
                    };
                    handle_loaded_character_data(self, entity, components, metadata);
                },
                ServerEvent::ExitIngame { entity } => {
                    handle_exit_ingame(self, entity, false);
                },
                ServerEvent::CreateNpc {
                    pos,
                    stats,
                    skill_set,
                    health,
                    poise,
                    inventory,
                    body,
                    agent,
                    alignment,
                    scale,
                    anchor: home_chunk,
                    loot,
                    rtsim_entity,
                    projectile,
                } => handle_create_npc(
                    self,
                    pos,
                    stats,
                    skill_set,
                    health,
                    poise,
                    inventory,
                    body,
                    agent,
                    alignment,
                    scale,
                    loot,
                    home_chunk,
                    rtsim_entity,
                    projectile,
                ),
                ServerEvent::CreateShip {
                    pos,
                    ship,
                    mountable,
                    agent,
                    rtsim_entity,
                } => handle_create_ship(self, pos, ship, mountable, agent, rtsim_entity),
                ServerEvent::CreateWaypoint(pos) => handle_create_waypoint(self, pos),
                ServerEvent::ClientDisconnect(entity, reason) => {
                    frontend_events.push(handle_client_disconnect(self, entity, reason, false))
                },
                ServerEvent::ClientDisconnectWithoutPersistence(entity) => {
                    frontend_events.push(handle_client_disconnect(
                        self,
                        entity,
                        common::comp::DisconnectReason::Kicked,
                        true,
                    ))
                },
                ServerEvent::Command(entity, name, args) => {
                    commands.push((entity, name, args));
                },
                ServerEvent::Chat(msg) => {
                    chat_messages.push(msg);
                },
                ServerEvent::Aura {
                    entity,
                    aura_change,
                } => handle_aura(self, entity, aura_change),
                ServerEvent::Buff {
                    entity,
                    buff_change,
                } => handle_buff(self, entity, buff_change),
                ServerEvent::EnergyChange { entity, change } => {
                    handle_energy_change(self, entity, change)
                },
                ServerEvent::ComboChange { entity, change } => {
                    handle_combo_change(self, entity, change)
                },
                ServerEvent::ParryHook { defender, attacker } => {
                    handle_parry_hook(self, defender, attacker)
                },
                ServerEvent::RequestSiteInfo { entity, id } => handle_site_info(self, entity, id),
                ServerEvent::MineBlock { entity, pos, tool } => {
                    handle_mine_block(self, entity, pos, tool)
                },
                ServerEvent::TeleportTo {
                    entity,
                    target,
                    max_range,
                } => handle_teleport_to(self, entity, target, max_range),
                ServerEvent::CreateSafezone { range, pos } => {
                    self.state.create_safezone(range, pos).build();
                },
                ServerEvent::Sound { sound } => handle_sound(self, &sound),
                ServerEvent::CreateSprite {
                    pos,
                    sprite,
                    del_timeout,
                } => handle_create_sprite(self, pos, sprite, del_timeout),
                ServerEvent::TamePet {
                    pet_entity,
                    owner_entity,
                } => handle_tame_pet(self, pet_entity, owner_entity),
                ServerEvent::EntityAttackedHook { entity } => {
                    handle_entity_attacked_hook(self, entity)
                },
                ServerEvent::ChangeAbility {
                    entity,
                    slot,
                    auxiliary_key,
                    new_ability,
                } => handle_change_ability(self, entity, slot, auxiliary_key, new_ability),
                ServerEvent::UpdateMapMarker { entity, update } => {
                    handle_update_map_marker(self, entity, update)
                },
                ServerEvent::MakeAdmin {
                    entity,
                    admin,
                    uuid,
                } => handle_make_admin(self, entity, admin, uuid),
                ServerEvent::ChangeStance { entity, stance } => {
                    handle_stance_change(self, entity, stance)
                },
            }
        }

        {
            let server_event_metrics = self
                .state
                .ecs()
                .read_resource::<crate::metrics::ServerEventMetrics>();
            event_counts
                .into_iter()
                .zip(ServerEventDiscriminants::VARIANTS)
                .for_each(|(count, event_name)| {
                    server_event_metrics
                        .event_count
                        .with_label_values(&[event_name])
                        .inc_by(count.into());
                })
        }

        for (entity, name, args) in commands {
            self.process_command(entity, name, args);
        }

        for msg in chat_messages {
            self.state.send_chat(msg);
        }

        frontend_events
    }
}
