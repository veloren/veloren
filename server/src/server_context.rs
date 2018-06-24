// Standard
use std::collections::hash_map::{Iter, IterMut};
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::time::Duration;

// Library
use bifrost::{Relay, event};
use config::PartialConfig;

// Project
use common::net::message::{ClientMessage, ServerMessage};
use common::Uid;
use region::Entity;

// Local
use player::Player;
use session::Session;

pub struct ServerContext {
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

impl ServerContext {
    pub fn new() -> ServerContext {
        ServerContext {
            // Config
            config: None,

            // Network
            listener_thread_handle: None,
            sessions: HashMap::new(),

            // Entities
            last_uid: 1,
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
    pub fn get_session(&self, id: u32) -> Option<&Session> { self.sessions.get(&id).map(|s| s.as_ref()) }
    pub fn get_session_mut(&mut self, id: u32) -> Option<&mut Session> { self.sessions.get_mut(&id).map(|s| s.as_mut()) }
    pub fn del_session(&mut self, id: u32) -> Option<Box<Session>> { self.sessions.remove(&id) }
    pub fn get_sessions(&self) -> Iter<u32, Box<Session>> { self.sessions.iter() }
    pub fn get_sessions_mut(&mut self) -> IterMut<u32, Box<Session>> { self.sessions.iter_mut() }

    // Entities

    pub fn add_entity(&mut self, id: Uid, entity: Box<Entity>) { self.entities.insert(id, entity); }
    pub fn get_entity(&mut self, id: Uid) -> Option<&mut Entity> { self.entities.get_mut(&id).map(|s| s.as_mut()) }
    pub fn del_entity(&mut self, id: Uid) -> Option<Box<Entity>> { self.entities.remove(&id) }
    pub fn get_entities(&self) -> Iter<Uid, Box<Entity>> { self.entities.iter() }
    pub fn get_entities_mut(&mut self) -> IterMut<Uid, Box<Entity>> { self.entities.iter_mut() }

    // Players

    pub fn add_player(&mut self, player: Box<Player>) { self.players.insert(player.get_uid(), player); }
    pub fn get_player(&self, id: Uid) -> Option<&Player> { self.players.get(&id).map(|s| s.as_ref()) }
    pub fn get_player_mut(&mut self, id: Uid) -> Option<&mut Player> { self.players.get_mut(&id).map(|s| s.as_mut()) }
    pub fn del_player(&mut self, id: Uid) -> Option<Box<Player>> { self.players.remove(&id) }
    pub fn get_players(&self) -> Iter<Uid, Box<Player>> { self.players.iter() }
    pub fn get_players_mut(&mut self) -> IterMut<Uid, Box<Player>> { self.players.iter_mut() }

    // Network

    pub fn send_message(&self, session_id: u32, message: ServerMessage) { self.get_session(session_id).map(|it| it.send_message(message)); }
    pub fn broadcast_packet(&self, message: ServerMessage) {
        self.sessions.iter().for_each(|(_, ref it)| it.send_message(message.clone()));
    }


    // Utils

    pub fn get_player_from_session(&self, session: &Session) -> Option<&Player> {
        if let Some(player_id) = session.get_player_id() {
            return self.get_player(player_id);
        }
        None
    }

    pub fn get_session_from_player(&mut self, player: &Player) -> Option<&mut Session> { self.get_session_mut(player.get_session_id()) }

    // Updates

    pub fn get_entity_updates(&self) -> Vec<(Uid, ServerMessage)> {
        self.get_entities()
            .map(|(entity_id, entity)| {
                (*entity_id, ServerMessage::EntityUpdate { uid: *entity_id, pos: entity.pos(), ori: entity.ori() })
            })
            .collect::<Vec<(Uid, ServerMessage)>>()
    }

    pub fn kick_session(&mut self, session_id: u32) {
        if let Some(session) = self.get_session(session_id) {
            info!("Session '{}' disconnected!", session_id);
            session.stop_conn();
            session.get_player_id().map(|player_id| self.kick_player(player_id));
        }
        self.del_session(session_id);
    }

    pub fn kick_player(&mut self, player_id: Uid) {
        if let Some(player) = self.get_player(player_id) {
            info!("Player '{}' disconnected!", player.alias());
            player.get_entity_uid().map(|entity_id| self.del_entity(entity_id));
        }
        self.del_player(player_id);
    }
}


pub const WORLD_UPDATE_TICK: u64 = 50;

pub fn update_world(relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
    //self.world.tick(dt); // TODO: Fix issue #11 and uncomment
    //debug!("TICK!");
    // Send Entity Updates

    debug!("Players Entities Sessions: {} {} {}", ctx.players.len(), ctx.entities.len(), ctx.sessions.len());

    remove_disconected_players(relay, ctx);
    send_entities_update(relay, ctx);


    relay.schedule(event(update_world), Duration::from_millis(WORLD_UPDATE_TICK));
}

fn remove_disconected_players(relay: &Relay<ServerContext>, ctx: &mut ServerContext) {

    let sessions_id_to_kick = ctx.get_sessions()
        .filter(|(_, session)| session.should_kick() )
        .map(|(session_id, _)| *session_id)
        .collect::<Vec<u32>>();

    for session_id in sessions_id_to_kick {
        ctx.kick_session(session_id);
    }
}

fn send_entities_update(relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
    let updates = ctx.get_entity_updates();
    let sessions = ctx.get_sessions();

    for (_, session) in sessions {
        let player = ctx.get_player_from_session(session.as_ref());
        match player.and_then(|p| p.get_entity_uid()) {
            Some(player_entity_id) => for (uid, update) in &updates {
                if *uid != player_entity_id {
                    let up = update.clone();
                    session.send_message(up);
                }
            },
            _ => {},
        }
    }
}
