use crate::{events::interaction::handle_tame_pet, state_ext::StateExt, Server};
use common::event::{EventBus, ServerEvent};
use common_base::span;
use entity_creation::{
    handle_beam, handle_create_npc, handle_create_ship, handle_create_waypoint,
    handle_initialize_character, handle_loaded_character_data, handle_shockwave, handle_shoot,
};
use entity_manipulation::{
    handle_aura, handle_bonk, handle_buff, handle_combo_change, handle_delete, handle_destroy,
    handle_energy_change, handle_entity_attacked_hook, handle_explosion, handle_health_change,
    handle_knockback, handle_land_on_ground, handle_parry, handle_poise, handle_respawn,
    handle_teleport_to,
};
use group_manip::handle_group;
use information::handle_site_info;
use interaction::{
    handle_create_sprite, handle_lantern, handle_mine_block, handle_mount, handle_npc_interaction,
    handle_possess, handle_sound, handle_unmount,
};
use inventory_manip::handle_inventory;
use invite::{handle_invite, handle_invite_response};
use player::{handle_client_disconnect, handle_exit_ingame};
use specs::{Builder, Entity as EcsEntity, WorldExt};
use trade::{cancel_trade_for, handle_process_trade_action};

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

        let mut requested_chunks = Vec::new();
        let mut commands = Vec::new();
        let mut chat_messages = Vec::new();

        let events = self
            .state
            .ecs()
            .read_resource::<EventBus<ServerEvent>>()
            .recv_all();

        for event in events {
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
                ServerEvent::PoiseChange {
                    entity,
                    change,
                    kb_dir,
                } => handle_poise(self, entity, change, kb_dir),
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
                } => handle_initialize_character(self, entity, character_id),
                ServerEvent::UpdateCharacterData { entity, components } => {
                    handle_loaded_character_data(self, entity, components);
                },
                ServerEvent::ExitIngame { entity } => {
                    cancel_trade_for(self, entity);
                    handle_exit_ingame(self, entity);
                },
                ServerEvent::CreateNpc {
                    pos,
                    stats,
                    skill_set,
                    health,
                    poise,
                    loadout,
                    body,
                    agent,
                    alignment,
                    scale,
                    anchor: home_chunk,
                    drop_item,
                    rtsim_entity,
                    projectile,
                } => handle_create_npc(
                    self,
                    pos,
                    stats,
                    skill_set,
                    health,
                    poise,
                    loadout,
                    body,
                    agent,
                    alignment,
                    scale,
                    drop_item,
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

                ServerEvent::ChunkRequest(entity, key) => {
                    requested_chunks.push((entity, key));
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
                ServerEvent::Parry {
                    entity,
                    energy_cost,
                } => handle_parry(self, entity, energy_cost),
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
                ServerEvent::CreateSprite { pos, sprite } => {
                    handle_create_sprite(self, pos, sprite)
                },
                ServerEvent::TamePet {
                    pet_entity,
                    owner_entity,
                } => handle_tame_pet(self, pet_entity, owner_entity),
                ServerEvent::EntityAttackedHook { entity } => {
                    handle_entity_attacked_hook(self, entity)
                },
            }
        }

        // Generate requested chunks
        for (entity, key) in requested_chunks {
            self.generate_chunk(entity, key);
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
