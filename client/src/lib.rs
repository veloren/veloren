#![feature(nll, euclidean_division)]

// Crates
#[macro_use]
extern crate log;
#[macro_use]
extern crate coord;
extern crate common;
extern crate region;

// Modules
mod player;
mod callbacks;
mod session;

// Reexport
pub use common::net::ClientMode;
pub use region::{Volume, Voxel, Chunk, Block, FnPayloadFunc};

// Constants
pub const CHUNK_SIZE: i64 = 32;

// Standard
use std::thread;
use std::time;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, Barrier};
use std::collections::HashMap;
use std::net::{ToSocketAddrs};

// Library
use coord::prelude::*;

// Project
use region::{Entity, VolMgr, VolGen, VolState};
use common::{get_version, Uid};
use common::net;
use common::net::{Connection, ServerMessage, ClientMessage, Callback, UdpMgr};

// Local
use player::Player;
use callbacks::Callbacks;

const VIEW_DISTANCE: i64 = 5;

// Errors that may occur within this crate
#[derive(Debug)]
pub enum Error {
    NetworkErr(net::Error),
}

impl From<net::Error> for Error {
    fn from(e: net::Error) -> Error {
        Error::NetworkErr(e)
    }
}

#[derive(PartialEq)]
pub enum ClientStatus {
    Connecting,
    Connected,
    Timeout,
    Disconnected,
}

pub trait Payloads: 'static {
    type Chunk: Send + Sync + 'static;
}

pub struct Client<P: Payloads> {
    status: RwLock<ClientStatus>,
    conn: Arc<Connection<ServerMessage>>,

    player: RwLock<Player>,
    entities: RwLock<HashMap<Uid, Entity>>,

    chunk_mgr: VolMgr<Chunk, <P as Payloads>::Chunk>,

    callbacks: RwLock<Callbacks>,

    finished: Barrier, // We use this to synchronize client shutdown
}

impl<P: Payloads> Callback<ServerMessage> for Client<P> {
    fn recv(&self, msg: Result<ServerMessage, common::net::Error>) {
        self.handle_packet(msg.unwrap());
    }
}

fn gen_chunk(pos: Vec2<i64>) -> Chunk {
    Chunk::test(vec3!(pos.x * CHUNK_SIZE, pos.y * CHUNK_SIZE, 0), vec3!(CHUNK_SIZE, CHUNK_SIZE, 128))
}

impl<P: Payloads> Client<P> {
    pub fn new<U: ToSocketAddrs, GF: FnPayloadFunc<Chunk, P::Chunk, Output=P::Chunk>>(mode: ClientMode, alias: String, remote_addr: U, gen_payload: GF) -> Result<Arc<Client<P>>, Error> {
        let conn = Connection::new::<U>(&remote_addr, Box::new(|_m| {}), None, UdpMgr::new())?;
        conn.send(ClientMessage::Connect{ mode, alias: alias.clone(), version: get_version() });
        Connection::start(&conn);

        let client = Arc::new(Client {
            status: RwLock::new(ClientStatus::Connecting),
            conn,

            player: RwLock::new(Player::new(alias)),
            entities: RwLock::new(HashMap::new()),

            chunk_mgr: VolMgr::new(CHUNK_SIZE, VolGen::new(gen_chunk, gen_payload)),

            callbacks: RwLock::new(Callbacks::new()),

            finished: Barrier::new(3),
        });

        *client.conn.callbackobj() = Some(client.clone());

        Self::start(client.clone());

        Ok(client)
    }

    fn set_status(&self, status: ClientStatus) {
        *self.status.write().unwrap() = status;
    }

    fn manage_chunks(&self) {
         // Generate terrain around the player
        if let Some(uid) = self.player().entity_uid {
            if let Some(player_entity) = self.entities_mut().get_mut(&uid) {
                let player_chunk = player_entity.pos()
                                                .map(|e| e as i64)
                                                .div_euc(vec3!([CHUNK_SIZE; 3]));

                // TODO: define a view distance?!
                for i in player_chunk.x - VIEW_DISTANCE .. player_chunk.x + VIEW_DISTANCE + 1 {
                    for j in player_chunk.y - VIEW_DISTANCE .. player_chunk.y + VIEW_DISTANCE + 1 {
                        if !self.chunk_mgr().contains(vec2!(i, j)) {
                            self.chunk_mgr().gen(vec2!(i, j));
                        }
                    }
                }

                // This should also be tied to view distance, and could be more efficient
                // (maybe? careful: deadlocks)
                let chunk_pos = self.chunk_mgr()
                    .volumes()
                    .keys()
                    .map(|p| *p)
                    .collect::<Vec<_>>();
                for pos in chunk_pos {
                    if (pos - vec2!(player_chunk.x, player_chunk.y)).snake_length() > VIEW_DISTANCE * 2 {
                        self.chunk_mgr().remove(pos);
                    }
                }
            }
        }
    }

    fn tick(&self, dt: f32) {
        if let Some(uid) = self.player().entity_uid {
            if let Some(player_entity) = self.entities_mut().get_mut(&uid) {
                let player_chunk = player_entity.pos()
                                                .map(|e| e as i64)
                                                .div_euc(vec3!([CHUNK_SIZE; 3]));

                // Gravity
                match self.chunk_mgr().at(vec2!(player_chunk.x, player_chunk.y)) {
                    Some(c) => match *c.read().unwrap() {
                        VolState::Exists(_, _) => player_entity.move_dir_mut().z -= 0.2,
                        _ => {},
                    }
                    None => {},
                }

                while self.chunk_mgr().get_voxel_at(
                    player_entity.pos().map(|e| e as i64)
                ).is_solid() {
                    player_entity.move_dir_mut().z = 0.0;
                    player_entity.pos_mut().z += 0.025;
                }

                self.conn.send(ClientMessage::PlayerEntityUpdate {
                    pos: player_entity.pos(),
                    move_dir: player_entity.move_dir(),
                    look_dir: player_entity.look_dir(),
                });
            }
        }

        for (uid, entity) in self.entities_mut().iter_mut() {
            let move_dir = entity.move_dir();
            *entity.pos_mut() += move_dir * dt;
        }
    }

    fn handle_packet(&self, packet: ServerMessage) {
        match packet {
            ServerMessage::Connected { entity_uid, version } => {
                if version == get_version() {
                    if let Some(uid) = entity_uid {
                        if !self.entities().contains_key(&uid) {
                            self.entities_mut().insert(uid, Entity::new(vec3!(0.0, 0.0, 0.0), vec3!(0.0, 0.0, 0.0), vec2!(0.0, 0.0)));
                        }
                    }
                    self.player_mut().entity_uid = entity_uid;
                    self.set_status(ClientStatus::Connected);
                    info!("Connected!");
                } else {
                    warn!("Server version mismatch: server is version {}. Disconnected.", version);
                    self.set_status(ClientStatus::Disconnected);
                }
            },
            ServerMessage::Kicked { reason } => {
                warn!("Server kicked client for {}", reason);
                self.set_status(ClientStatus::Disconnected);
            }
            ServerMessage::Shutdown => self.set_status(ClientStatus::Disconnected),
            ServerMessage::RecvChatMsg { alias, msg } => self.callbacks().call_recv_chat_msg(&alias, &msg),
            ServerMessage::EntityUpdate { uid, pos, move_dir, look_dir } => {
                info!("Entity Update: uid:{} at pos:{:#?}, move_dir:{:#?}, look_dir:{:#?}", uid, pos, move_dir, look_dir);

                let mut entities = self.entities_mut();
                match entities.get_mut(&uid) {
                    Some(e) => {
                        *e.pos_mut() = pos;
                        *e.move_dir_mut() = move_dir;
                        *e.look_dir_mut() = look_dir;
                    }
                    None => { entities.insert(uid, Entity::new(pos, move_dir, look_dir)); },
                }
            },
            ServerMessage::Ping => self.conn.send(ClientMessage::Ping),
            _ => {},
        }
    }

    fn start(client: Arc<Client<P>>) {

        let client_ref = client.clone();
        thread::spawn(move || {
            while *client_ref.status() != ClientStatus::Disconnected {
                client_ref.tick(0.2);
                thread::sleep(time::Duration::from_millis(20));
            }
            // Notify anything else that we've finished ticking
            client_ref.finished.wait();
        });

        thread::spawn(move || {
            while *client.status() != ClientStatus::Disconnected {
                client.manage_chunks();
                thread::sleep(time::Duration::from_millis(100));
            }
            // Notify anything else that we've finished ticking
            client.finished.wait();
        });
    }

    // Public interface

    pub fn shutdown(&self) {
        self.conn.send(ClientMessage::Disconnect);
        self.set_status(ClientStatus::Disconnected);
        self.finished.wait();
        //thread::sleep(time::Duration::from_millis(50)); // workaround for making sure that networking sends the Disconnect Msg
    }

    pub fn send_chat_msg(&self, msg: String) -> Result<(), Error> {
        Ok(self.conn.send(ClientMessage::ChatMsg { msg }))
    }

    pub fn send_cmd(&self, cmd: String) -> Result<(), Error> {
        Ok(self.conn.send(ClientMessage::SendCmd { cmd }))
    }

    pub fn chunk_mgr<'a>(&'a self) -> &'a VolMgr<Chunk, P::Chunk> { &self.chunk_mgr }

    pub fn status<'a>(&'a self) -> RwLockReadGuard<'a, ClientStatus> { self.status.read().unwrap() }

    pub fn callbacks<'a>(&'a self) -> RwLockReadGuard<'a, Callbacks> { self.callbacks.read().unwrap() }

    pub fn player<'a>(&'a self) -> RwLockReadGuard<'a, Player> { self.player.read().unwrap() }
    pub fn player_mut<'a>(&'a self) -> RwLockWriteGuard<'a, Player> { self.player.write().unwrap() }

    pub fn entities<'a>(&'a self) -> RwLockReadGuard<'a, HashMap<Uid, Entity>> { self.entities.read().unwrap() }
    pub fn entities_mut<'a>(&'a self) -> RwLockWriteGuard<'a, HashMap<Uid, Entity>> { self.entities.write().unwrap() }
}
