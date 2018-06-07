#![feature(nll)]

#[macro_use]
extern crate log;
extern crate common;
extern crate network;
extern crate region;
extern crate nalgebra;

mod player;
mod callbacks;

// Reexports
pub use network::ClientMode as ClientMode;
pub use region::Volume as Volume;

use std::thread;
use std::time;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, Barrier};
use std::collections::HashMap;
use std::net::ToSocketAddrs;

use nalgebra::Vector3;

use network::client::ClientConn;
use network::packet::{ClientPacket, ServerPacket};
use region::Entity;
use common::{get_version, Uid};

use player::Player;
use callbacks::Callbacks;

// Errors that may occur within this crate
#[derive(Debug)]
pub enum Error {
    NetworkErr(network::Error),
}

impl From<network::Error> for Error {
    fn from(e: network::Error) -> Error {
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
    conn: ClientConn,

    player: RwLock<Player>,
    entities: RwLock<HashMap<Uid, Entity>>,

    callbacks: RwLock<Callbacks>,

    finished: Barrier, // We use this to synchronize client shutdown
}

impl Client {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(mode: ClientMode, alias: String, bind_addr: T, remote_addr: U) -> Result<Arc<Client>, Error> {
        let conn = ClientConn::new(bind_addr, remote_addr)?;
        conn.send(&ClientPacket::Connect{ mode, alias: alias.to_string(), version: get_version() })?;

        let client = Arc::new(Client {
            status: RwLock::new(ClientStatus::Connecting),
            conn,

            player: RwLock::new(Player::new(alias)),
            entities: RwLock::new(HashMap::new()),

            callbacks: RwLock::new(Callbacks::new()),

            finished: Barrier::new(2),
        });

        Self::start(client.clone());

        Ok(client)
    }

    fn conn<'a>(&'a self) -> &'a ClientConn {
        &self.conn
    }

    fn set_status(&self, status: ClientStatus) {
        *self.status.write().unwrap() = status;
    }

    fn tick(&self, dt: f32) {
        if let Some(uid) = self.player().entity_uid {
            if let Some(e) = self.entities_mut().get_mut(&uid) {
                *e.pos_mut() += self.player().dir_vec * dt;

                self.conn.send(&ClientPacket::PlayerEntityUpdate {
                    pos: *e.pos()
                }).expect("Could not send player position packet");
            }
        }
    }

    fn handle_packet(&self, packet: ServerPacket) {
        match packet {
            ServerPacket::Connected { player_entity_uid, version } => {
                match version == get_version() {
                    true => {
                        if let Some(uid) = player_entity_uid {
                            if !self.entities().contains_key(&uid) {
                                self.entities_mut().insert(uid, Entity::new(Vector3::new(0.0, 0.0, 0.0)));
                            }
                        }

                        self.player_mut().entity_uid = player_entity_uid;

                        info!("Client connected");
                    },
                    false => {
                        warn!("Server version mismatch: server is version {}. Disconnected.", version);
                        self.set_status(ClientStatus::Disconnected);
                    }
                }
                self.set_status(ClientStatus::Connected);
            },
            ServerPacket::Kicked { reason } => {
                warn!("Server kicked client for {}", reason);
                self.set_status(ClientStatus::Disconnected);
            }
            ServerPacket::Shutdown => self.set_status(ClientStatus::Disconnected),
            ServerPacket::RecvChatMsg { alias, msg } => self.callbacks().call_recv_chat_msg(&alias, &msg),
            ServerPacket::EntityUpdate { uid, pos } => {
                info!("Entity Update: uid:{} at pos:{:#?}", uid, pos);

                let mut entities = self.entities_mut();
                match entities.get_mut(&uid) {
                    Some(e) => *e.pos_mut() = pos,
                    None => { entities.insert(uid, Entity::new(pos)); },
                }
            },
            _ => {},
        }
    }

    fn start(client: Arc<Client>) {
        let client_ref = client.clone();
        thread::spawn(move || {
            while *client_ref.status() != ClientStatus::Disconnected {
                match client_ref.conn().recv() {
                    Ok(data) => client_ref.handle_packet(data.1),
                    Err(e) => warn!("Receive error: {:?}", e),
                }
            }
            // Notify anything else that we've finished networking
            client_ref.finished.wait();
        });

        let client_ref = client.clone();
        thread::spawn(move || {
            while *client_ref.status() != ClientStatus::Disconnected {
                client.tick(0.2);
                thread::sleep(time::Duration::from_millis(20));
            }
            // Notify anything else that we've finished ticking
            client_ref.finished.wait();
        });
    }

    // Public interface

    pub fn shutdown(&self) {
        self.conn.send(&ClientPacket::Disconnect).expect("Could not send disconnect packet");
        self.set_status(ClientStatus::Disconnected);
        self.finished.wait();
    }

    pub fn send_chat_msg(&self, msg: &str) -> Result<(), Error> {
        self.conn.send(&ClientPacket::SendChatMsg{
            msg: msg.to_string(),
        })?;
        Ok(())
    }

    pub fn send_cmd(&self, cmd: &str) -> Result<(), Error> {
        self.conn.send(&ClientPacket::SendCmd{
            cmd: cmd.to_string(),
        })?;
        Ok(())
    }

    pub fn status<'a>(&'a self) -> RwLockReadGuard<'a, ClientStatus> { self.status.read().unwrap() }

    pub fn callbacks<'a>(&'a self) -> RwLockReadGuard<'a, Callbacks> { self.callbacks.read().unwrap() }

    pub fn player<'a>(&'a self) -> RwLockReadGuard<'a, Player> { self.player.read().unwrap() }
    pub fn player_mut<'a>(&'a self) -> RwLockWriteGuard<'a, Player> { self.player.write().unwrap() }

    pub fn entities<'a>(&'a self) -> RwLockReadGuard<'a, HashMap<Uid, Entity>> { self.entities.read().unwrap() }
    pub fn entities_mut<'a>(&'a self) -> RwLockWriteGuard<'a, HashMap<Uid, Entity>> { self.entities.write().unwrap() }
}
