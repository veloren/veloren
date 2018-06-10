use bifrost::relay::Relay;
use bifrost::event::event;
use common::network::packet::{ClientPacket, ServerPacket};
use common::Uid;
use config::PartialConfig;
use player::Player;
use region::Entity;
use session::Session;
use std::collections::hash_map::{Iter, IterMut};
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct World {
    // Configuration
    config: Option<PartialConfig>,

    // Network
    listener_thread_handle: Option<JoinHandle<()>>,
    sessions: HashMap<u32, Box<Session>>,

    // Entities$
    last_uid: Uid,
    entities: HashMap<Uid, Box<Entity>>,
    players: HashMap<Uid, Box<Player>>,
}

impl World {
    pub fn new() -> World {
        World {
            // Config
            config: None,

            // Network
            listener_thread_handle: None,
            sessions: HashMap::new(),

            // Entities
            last_uid: 0,
            entities: HashMap::new(),
            players: HashMap::new(),
        }
    }

    // Entities

    pub fn new_uid(&mut self) -> Uid {
        self.last_uid += 1;
        self.last_uid
    }

    // Sessions

    pub fn add_session(&mut self, session: Box<Session>) { self.sessions.insert(session.get_id(), session); }
    pub fn get_session(&mut self, id: u32) -> Option<&mut Session> { self.sessions.get_mut(&id).map(|s| s.as_mut()) }
    pub fn del_session(&mut self, id: u32) -> Option<Box<Session>> { self.sessions.remove(&id) }
    pub fn get_sessions(&mut self) -> Iter<u32, Box<Session>> { self.sessions.iter() }
    pub fn get_sessions_mut(&mut self) -> IterMut<u32, Box<Session>> { self.sessions.iter_mut() }

    // Entities

    pub fn add_entity(&mut self, id: Uid, entity: Box<Entity>) { self.entities.insert(id, entity); }
    pub fn get_entity(&mut self, id: Uid) -> Option<&mut Entity> { self.entities.get_mut(&id).map(|s| s.as_mut()) }
    pub fn del_entity(&mut self, id: Uid) -> Option<Box<Entity>> { self.entities.remove(&id) }
    pub fn get_entities(&mut self) -> Iter<Uid, Box<Entity>> { self.entities.iter() }
    pub fn get_entities_mut(&mut self) -> IterMut<Uid, Box<Entity>> { self.entities.iter_mut() }

    // Players

    pub fn add_player(&mut self, player: Box<Player>) { self.players.insert(player.get_uid(), player); }
    pub fn get_player(&mut self, id: Uid) -> Option<&mut Player> { self.players.get_mut(&id).map(|s| s.as_mut()) }
    pub fn del_player(&mut self, id: Uid) -> Option<Box<Player>> { self.players.remove(&id) }
    pub fn get_players(&mut self) -> Iter<Uid, Box<Player>> { self.players.iter() }
    pub fn get_players_mut(&mut self) -> IterMut<Uid, Box<Player>> { self.players.iter_mut() }

    // Network

    pub fn send_packet(&mut self, session_id: u32, packet: &ServerPacket) { self.get_session(session_id).map(|it| it.get_handler().send_packet(packet)); }
    pub fn broadcast_packet(&mut self, packet: &ServerPacket) {
        self.sessions.iter_mut().for_each(|(_, ref mut it)| it.get_handler().send_packet(packet).unwrap());
    }
}


pub const WORLD_UPDATE_TICK: u64 = 50;

pub fn update_world(relay: &Relay<World>, ctx: &mut World) {
    //self.world.tick(dt); // TODO: Fix issue #11 and uncomment
    //println!("TICK!");
    // Send Entity Updates

    let updates = ctx.get_entities().map(|(entity_id, entity)| {
        ServerPacket::EntityUpdate { uid: *entity_id, pos: *entity.pos() }
    }).collect::<Vec<ServerPacket>>();

    ctx.get_sessions_mut().for_each(|(_, session)| {
        if session.has_player() {
            updates.iter().for_each(|update| {
                session.get_handler().send_packet(update);
            })
        }
    });

    relay.schedule(event(update_world), Duration::from_millis(WORLD_UPDATE_TICK));
}
