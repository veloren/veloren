#![feature(nll)]

#[macro_use]
extern crate log;
extern crate common;
extern crate region;
extern crate nalgebra;
extern crate byteorder;

mod player;
mod callbacks;
mod session;

// Reexports
pub use common::network::ClientMode as ClientMode;
pub use region::Volume as Volume;

use std::thread;
use std::time;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, Barrier};
use std::collections::HashMap;
use std::net::ToSocketAddrs;

use nalgebra::Vector3;

use common::network::client::ClientConn;
use common::network::packet::{ClientPacket, ServerPacket};
use region::Entity;
use common::{get_version, Uid};

use player::Player;
use callbacks::Callbacks;
use std::net::TcpStream;
use session::Session;
use std::sync::Mutex;

// Errors that may occur within this crate
#[derive(Debug)]
pub enum Error {
    NetworkErr(common::network::Error),
}

impl From<common::network::Error> for Error {
    fn from(e: common::network::Error) -> Error {
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
    session: Session,

    player: RwLock<Player>,
    entities: RwLock<HashMap<Uid, Entity>>,

    callbacks: RwLock<Callbacks>,

    finished: Barrier, // We use this to synchronize client shutdown
}

impl Client {
    pub fn new<U: ToSocketAddrs>(mode: ClientMode, alias: String, remote_addr: U) -> Result<Arc<RwLock<Client>>, Error> {
        let stream = TcpStream::connect(remote_addr).unwrap();
        let mut session = Session::new(stream);
        session.send_packet(&ClientPacket::Connect{ mode, alias: alias.to_string(), version: get_version() });

        let client = Arc::new(RwLock::new(Client {
            status: RwLock::new(ClientStatus::Connecting),
            session,

            player: RwLock::new(Player::new(alias)),
            entities: RwLock::new(HashMap::new()),

            callbacks: RwLock::new(Callbacks::new()),

            finished: Barrier::new(2),
        }));

        Self::start(client.clone());

        Ok(client)
    }

    fn session<'a>(&'a self) -> &'a Session {
        &self.session
    }

    fn set_status(&self, status: ClientStatus) {
        *self.status.write().unwrap() = status;
    }

    fn tick(&mut self, dt: f32) {

        let packet = if let Some(uid) = self.player().entity_uid {
            if let Some(e) = self.entities_mut().get_mut(&uid) {
                *e.pos_mut() += self.player().dir_vec * dt;

                Some(ClientPacket::PlayerEntityUpdate {
                    pos: *e.pos()
                })
            } else { None }
        } else { None };

        packet.map(|it| self.session.send_packet(&it).expect("Could not send player position packet"));
    }

    fn handle_packet(&self, packet: ServerPacket) {
        match packet {
            ServerPacket::Connected { entity_uid, version } => {
                match version == get_version() {
                    true => {
                        if let Some(uid) = entity_uid {
                            if !self.entities().contains_key(&uid) {
                                self.entities_mut().insert(uid, Entity::new(Vector3::new(0.0, 0.0, 0.0)));
                            }
                        }

                        self.player_mut().entity_uid = entity_uid;

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

    fn start(client: Arc<RwLock<Client>>) {
        let mut client_ref = client.clone();
        thread::spawn(move || {
            let mut client_ref = client_ref.write().unwrap();
            loop {
                if *client_ref.status() != ClientStatus::Disconnected {
                    break;
                }
                match client_ref.session.recv_packet() {
                    Ok(data) => client_ref.handle_packet(data),
                    Err(e) => warn!("Receive error: {:?}", e),
                }
            }
            // Notify anything else that we've finished networking
            client_ref.finished.wait();
        });

        let mut client_ref = client.clone();
        thread::spawn(move || {
            let mut client_ref = client_ref.write().unwrap();
            loop {
                if *client_ref.status() != ClientStatus::Disconnected {
                    break;
                }
                client_ref.tick(0.2);
                thread::sleep(time::Duration::from_millis(20));
            }
            // Notify anything else that we've finished ticking
            client_ref.finished.wait();
        });
    }

    // Public interface

    pub fn shutdown(&mut self) {
        self.session.send_packet(&ClientPacket::Disconnect).expect("Could not send disconnect packet");
        self.set_status(ClientStatus::Disconnected);
        self.finished.wait();
    }

    pub fn send_chat_msg(&mut self, msg: &str) -> Result<(), Error> {
        self.session.send_packet(&ClientPacket::ChatMsg {
            msg: msg.to_string(),
        });
        Ok(())
    }

    pub fn send_cmd(&mut self, cmd: &str) -> Result<(), Error> {
        self.session.send_packet(&ClientPacket::SendCmd{
            cmd: cmd.to_string(),
        });
        Ok(())
    }

    pub fn status<'a>(&'a self) -> RwLockReadGuard<'a, ClientStatus> { self.status.read().unwrap() }

    pub fn callbacks<'a>(&'a self) -> RwLockReadGuard<'a, Callbacks> { self.callbacks.read().unwrap() }

    pub fn player<'a>(&'a self) -> RwLockReadGuard<'a, Player> { self.player.read().unwrap() }
    pub fn player_mut<'a>(&'a self) -> RwLockWriteGuard<'a, Player> { self.player.write().unwrap() }

    pub fn entities<'a>(&'a self) -> RwLockReadGuard<'a, HashMap<Uid, Entity>> { self.entities.read().unwrap() }
    pub fn entities_mut<'a>(&'a self) -> RwLockWriteGuard<'a, HashMap<Uid, Entity>> { self.entities.write().unwrap() }
}
