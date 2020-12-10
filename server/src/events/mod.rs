use crate::{state_ext::StateExt, Server};
use common::{
    event::{EventBus, ServerEvent},
    span,
};
use entity_creation::{
    handle_beam, handle_create_npc, handle_create_waypoint, handle_initialize_character,
    handle_loaded_character_data, handle_shockwave, handle_shoot,
};
use entity_manipulation::{
    handle_aura, handle_buff, handle_damage, handle_delete, handle_destroy, handle_energy_change,
    handle_explosion, handle_knockback, handle_land_on_ground, handle_poise, handle_respawn,
};
use group_manip::handle_group;
use interaction::{handle_lantern, handle_mount, handle_possess, handle_unmount};
use inventory_manip::handle_inventory;
use player::{handle_client_disconnect, handle_exit_ingame};
use specs::{Entity as EcsEntity, WorldExt};

mod entity_creation;
mod entity_manipulation;
mod group_manip;
mod interaction;
mod inventory_manip;
mod player;

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
        let mut chat_commands = Vec::new();
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
                    reagent,
                } => handle_explosion(&self, pos, explosion, owner, reagent),
                ServerEvent::Shoot {
                    entity,
                    dir,
                    body,
                    light,
                    projectile,
                    gravity,
                    speed,
                } => handle_shoot(self, entity, dir, body, light, projectile, gravity, speed),
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
                    handle_knockback(&self, entity, impulse)
                },
                ServerEvent::Damage { entity, change } => handle_damage(&self, entity, change),
                ServerEvent::PoiseChange {
                    entity,
                    change,
                    kb_dir,
                } => handle_poise(&self, entity, change, kb_dir),
                ServerEvent::Delete(entity) => handle_delete(self, entity),
                ServerEvent::Destroy { entity, cause } => handle_destroy(self, entity, cause),
                ServerEvent::InventoryManip(entity, manip) => handle_inventory(self, entity, manip),
                ServerEvent::GroupManip(entity, manip) => handle_group(self, entity, manip),
                ServerEvent::Respawn(entity) => handle_respawn(&self, entity),
                ServerEvent::LandOnGround { entity, vel } => {
                    handle_land_on_ground(&self, entity, vel)
                },
                ServerEvent::EnableLantern(entity) => handle_lantern(self, entity, true),
                ServerEvent::DisableLantern(entity) => handle_lantern(self, entity, false),
                ServerEvent::Mount(mounter, mountee) => handle_mount(self, mounter, mountee),
                ServerEvent::Unmount(mounter) => handle_unmount(self, mounter),
                ServerEvent::Possess(possessor_uid, possesse_uid) => {
                    handle_possess(&self, possessor_uid, possesse_uid)
                },
                ServerEvent::InitCharacterData {
                    entity,
                    character_id,
                } => handle_initialize_character(self, entity, character_id),
                ServerEvent::UpdateCharacterData { entity, components } => {
                    handle_loaded_character_data(self, entity, components);
                },
                ServerEvent::ExitIngame { entity } => handle_exit_ingame(self, entity),
                ServerEvent::CreateNpc {
                    pos,
                    stats,
                    health,
                    poise,
                    loadout,
                    body,
                    agent,
                    alignment,
                    scale,
                    home_chunk,
                    drop_item,
                    rtsim_entity,
                } => handle_create_npc(
                    self,
                    pos,
                    stats,
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
                ),
                ServerEvent::CreateWaypoint(pos) => handle_create_waypoint(self, pos),
                ServerEvent::ClientDisconnect(entity) => {
                    frontend_events.push(handle_client_disconnect(self, entity))
                },

                ServerEvent::ChunkRequest(entity, key) => {
                    requested_chunks.push((entity, key));
                },
                ServerEvent::ChatCmd(entity, cmd) => {
                    chat_commands.push((entity, cmd));
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
                    handle_energy_change(&self, entity, change)
                },
            }
        }

        // Generate requested chunks.
        for (entity, key) in requested_chunks {
            self.generate_chunk(entity, key);
        }

        for (entity, cmd) in chat_commands {
            self.process_chat_cmd(entity, cmd);
        }

        for msg in chat_messages {
            self.state.send_chat(msg);
        }

        frontend_events
    }
}
