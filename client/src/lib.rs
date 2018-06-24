#![feature(nll)]

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
pub use common::net::ClientMode as ClientMode;
pub use region::Volume as Volume;

// Standard
use std::thread;
use std::time;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, Barrier};
use std::collections::HashMap;
use std::net::{ToSocketAddrs, TcpStream};

// Library
use coord::prelude::*;

// Project
use region::Entity;
use common::{get_version, Uid};
use common::net;
use common::net::{Connection, ServerMessage, ClientMessage, Callback, UdpMgr};

// Local
use player::Player;
use callbacks::Callbacks;

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

pub struct Client {
    status: RwLock<ClientStatus>,
    conn: Arc<Connection<ServerMessage>>,

    player: RwLock<Player>,
    entities: RwLock<HashMap<Uid, Entity>>,

    callbacks: RwLock<Callbacks>,

    finished: Barrier, // We use this to synchronize client shutdown
}

impl Callback<ServerMessage> for Client {
    fn recv(&self, msg: Box<Result<ServerMessage, common::net::Error>>) {
        self.handle_packet(msg.unwrap());
    }
}

impl Client {
    pub fn new<U: ToSocketAddrs>(mode: ClientMode, alias: String, remote_addr: U) -> Result<Arc<Client>, Error> {
        let mut conn = Connection::new::<U>(&remote_addr, Box::new(|m| {
            //
            //self.handle_packet(m);
        }), None, UdpMgr::new())?;
        conn.send(ClientMessage::Connect{ mode, alias: alias.clone(), version: get_version() });
        Connection::start(&conn);

        let client = Arc::new(Client {
            status: RwLock::new(ClientStatus::Connecting),
            conn,

            player: RwLock::new(Player::new(alias)),
            entities: RwLock::new(HashMap::new()),

            callbacks: RwLock::new(Callbacks::new()),

            finished: Barrier::new(2),
        });

        *client.conn.callbackobj() = Some(Box::new(client.clone()));

        /*
        *client.conn.callback() = Box::new(|m: Box<ServerMessage>| {
            client.handle_packet(*m);
        });*/

        Self::start(client.clone());

        Ok(client)
    }

    fn set_status(&self, status: ClientStatus) {
        *self.status.write().unwrap() = status;
    }

    fn tick(&self, dt: f32) {
        if let Some(uid) = self.player().entity_uid {
            if let Some(e) = self.entities_mut().get_mut(&uid) {
                *e.pos_mut() += self.player().dir_vec * dt;

                self.conn.send(ClientMessage::PlayerEntityUpdate {
                    pos: e.pos(),
                    ori: e.ori(),
                });
            }
        }
    }

    fn handle_packet(&self, packet: ServerMessage) {
        match packet {
            ServerMessage::Connected { entity_uid, version } => {
                if version == get_version() {
                    if let Some(uid) = entity_uid {
                        if !self.entities().contains_key(&uid) {
                            self.entities_mut().insert(uid, Entity::new(vec3!(0.0, 0.0, 0.0), 0.0));
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
            ServerMessage::EntityUpdate { uid, pos, ori } => {
                info!("Entity Update: uid:{} at pos:{:#?}", uid, pos);

                let mut entities = self.entities_mut();
                match entities.get_mut(&uid) {
                    Some(e) => *e.pos_mut() = pos,
                    None => { entities.insert(uid, Entity::new(pos, ori)); },
                }
            },
            _ => {},
        }
    }

    fn start(client: Arc<Client>) {
        /*let client_ref = client.clone();

        thread::spawn(move || {
            while *client_ref.status() != ClientStatus::Disconnected {
                match client_ref.mngr.recv() {
                    Ok(p) => client_ref.handle_packet(p),
                    Err(e) => warn!("Receive error: {:?}", e),
                }
            }
            // Notify anything else that we've finished networking
            client_ref.finished.wait();
        });
        */


        let client_ref = client.clone();
        thread::spawn(move || {
            while *client_ref.status() != ClientStatus::Disconnected {
                client_ref.tick(0.2);
                thread::sleep(time::Duration::from_millis(20));
            }
            // Notify anything else that we've finished ticking
            client_ref.finished.wait();
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

    pub fn status<'a>(&'a self) -> RwLockReadGuard<'a, ClientStatus> { self.status.read().unwrap() }

    pub fn callbacks<'a>(&'a self) -> RwLockReadGuard<'a, Callbacks> { self.callbacks.read().unwrap() }

    pub fn player<'a>(&'a self) -> RwLockReadGuard<'a, Player> { self.player.read().unwrap() }
    pub fn player_mut<'a>(&'a self) -> RwLockWriteGuard<'a, Player> { self.player.write().unwrap() }

    pub fn entities<'a>(&'a self) -> RwLockReadGuard<'a, HashMap<Uid, Entity>> { self.entities.read().unwrap() }
    pub fn entities_mut<'a>(&'a self) -> RwLockWriteGuard<'a, HashMap<Uid, Entity>> { self.entities.write().unwrap() }
}
