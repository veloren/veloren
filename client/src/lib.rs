#![feature(nll)]

#[macro_use]
extern crate log;
extern crate common;
extern crate spin;
extern crate network;
extern crate region;
extern crate nalgebra;

// Reexports
pub use network::ClientMode as ClientMode;
pub use region::Volume as Volume;

use std::thread;
use std::thread::JoinHandle;
use std::time;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::net::ToSocketAddrs;

use spin::{Mutex, MutexGuard, RwLock, RwLockReadGuard};

use network::client::ClientConn;
use network::packet::{ClientPacket, ServerPacket};
use region::Entity;

use common::get_version;

use nalgebra::{Vector3};

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

pub struct Client {
    running: AtomicBool,
    conn: ClientConn,
    alias: Mutex<String>,

    player_entity_uid: Mutex<Option<u64>>, // TODO: Turn u64 into Uid
    entities: RwLock<HashMap<u64, Entity>>, // TODO: Turn u64 into Uid
    player_vel: Mutex<Vector3<f32>>,

    chat_callback: Mutex<Option<Box<Fn(&str, &str) + Send>>>,
}

impl Client {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(mode: ClientMode, alias: String, bind_addr: T, remote_addr: U) -> Result<Arc<Client>, Error> {
        let conn = ClientConn::new(bind_addr, remote_addr)?;
        conn.send(&ClientPacket::Connect{ mode, alias: alias.to_string(), version: get_version() })?;

        Ok(Arc::new(Client {
            running: AtomicBool::new(true),
            conn,
            alias: Mutex::new(alias),

            player_entity_uid: Mutex::new(None),
            entities: RwLock::new(HashMap::new()),
            player_vel: Mutex::new(Vector3::new(0.0, 0.0, 0.0)),

            chat_callback: Mutex::new(None),
        }))
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn entities<'a>(&'a self) -> RwLockReadGuard<'a, HashMap<u64, Entity>> {
        self.entities.read()
    }

    pub fn player_entity_uid<'a>(&'a self) -> Option<u64> {
        *self.player_entity_uid.lock()
    }

    pub fn alias_mut<'a>(&'a self) -> MutexGuard<'a, String> {
        self.alias.lock()
    }

    pub fn conn<'a>(&'a self) -> &'a ClientConn {
        &self.conn
    }

    pub fn set_player_vel(&self, vel: Vector3<f32>) {
        *self.player_vel.lock() = vel;
    }

    fn tick(&self, dt: f32) {
        if let Some(uid) = *self.player_entity_uid.lock() {
            if let Some(e) = self.entities.write().get_mut(&uid) {
                *e.pos_mut() += *self.player_vel.lock() * dt;

                self.conn.send(&ClientPacket::PlayerEntityUpdate {
                    pos: *e.pos()
                }).expect("Could not send player position packet");
            }
        }
    }

    pub fn handle_packet(&self, packet: ServerPacket) {
        match packet {
            ServerPacket::Connected { player_entity_uid, version } => {
                match version == get_version() {
                    true => {
                        if let Some(uid) = player_entity_uid {
                            if !self.entities.read().contains_key(&uid) {
                                self.entities.write().insert(uid, Entity::new(Vector3::new(0.0, 0.0, 0.0)));
                            }
                        }

                        *self.player_entity_uid.lock() = player_entity_uid;

                        info!("Client connected");
                    },
                    false => {
                        warn!("Server version mismatch, Server is version {} exitting...", version);
                        self.running.store(false, Ordering::Relaxed)
                    }
                }
                
            },
            ServerPacket::Kicked { reason } => {
                warn!("Server kicked client for {}", reason);
                self.running.store(false, Ordering::Relaxed)
            }
            ServerPacket::Shutdown => self.running.store(false, Ordering::Relaxed),
            ServerPacket::RecvChatMsg { alias, msg } => match *self.chat_callback.lock() {
                Some(ref f) => (f)(&alias, &msg),
                None => {}
            },
            ServerPacket::EntityUpdate { uid, pos } => {
                info!("Entity Update: uid:{} at pos:{:#?}", uid, pos);

                let mut entities = self.entities.write();
                match entities.get_mut(&uid) {
                    Some(e) => *e.pos_mut() = pos,
                    None => { entities.insert(uid, Entity::new(pos)); },
                }
            },
            _ => {},
        }
    }

    pub fn set_chat_callback<F: 'static + Fn(&str, &str) + Send>(&self, f: F) {
        *self.chat_callback.lock() = Some(Box::new(f));
    }

    pub fn send_chat_msg(&self, msg: &str) -> Result<(), Error> {
        self.conn.send(&ClientPacket::SendChatMsg{
            msg: msg.to_string(),
        })?;
        Ok(())
    }

    pub fn send_command(&self, cmd: &str) -> Result<(), Error> {
        self.conn.send(&ClientPacket::SendCommand{
            cmd: cmd.to_string(),
        })?;
        Ok(())
    }

    pub fn stop(client: Arc<Client>) {
        client.running.store(false, Ordering::Relaxed);
    }

    // Todo: stop this being run twice?
    pub fn start(client: Arc<Client>) {
        let client_ref = client.clone();
        let recv_thread = thread::spawn(move || {
            while client_ref.running() {
                match client_ref.conn().recv() {
                    Ok(data) => client_ref.handle_packet(data.1),
                    Err(e) => warn!("Receive error: {:?}", e),
                }
            }
        });

        let client_ref = client.clone();
        let tick_thread = thread::spawn(move || {
            while client_ref.running() {
                client.tick(0.2);
                thread::sleep(time::Duration::from_millis(20));
            }
        });

        // TODO: Make these join the main thread before closing!
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.conn.send(&ClientPacket::Disconnect).expect("Could not send disconnect packet");
    }
}
