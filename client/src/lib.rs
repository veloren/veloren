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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::net::ToSocketAddrs;

use spin::{Mutex, MutexGuard, RwLock, RwLockReadGuard};

use network::client::ClientConn;
use network::packet::{ClientPacket, ServerPacket};
use region::Entity;

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
    player_movement: Mutex<Vector3<f32>>,

    chat_callback: Mutex<Option<Box<Fn(&str, &str) + Send>>>,
}

impl Client {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(mode: ClientMode, alias: String, bind_addr: T, remote_addr: U) -> Result<Arc<Client>, Error> {
        let conn = ClientConn::new(bind_addr, remote_addr)?;
        conn.send(&ClientPacket::Connect{ mode, alias: alias.to_string() })?;

        Ok(Arc::new(Client {
            running: AtomicBool::new(true),
            conn,
            alias: Mutex::new(alias),

            player_entity_uid: Mutex::new(None),
            entities: RwLock::new(HashMap::new()),
            player_movement: Mutex::new(Vector3::new(0.0, 0.0, 0.0)),

            chat_callback: Mutex::new(None),
        }))
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn entities<'a>(&'a self) -> RwLockReadGuard<'a, HashMap<u64, Entity>> {
        self.entities.read()
    }

    pub fn player_entity_uid<'a>(&'a self) -> MutexGuard<'a, Option<u64>> {
        self.player_entity_uid.lock()
    }

    // sets the movement independent from current looking position
    pub fn set_absolute_movement<'a>(&'a self, movement: Vector3<f32>) {
        *self.player_movement.lock() = movement;
    }

    // sets the movement independent relative to the current looking position
    pub fn set_relative_movement<'a>(&'a self, movement: Vector3<f32>) {
        // rotate movement first
        *self.player_movement.lock() = movement;
    }

    fn move_player<'a>(&'a self) {
        let uid = self.player_entity_uid.lock().unwrap();
        let mut entities = self.entities.write();
        match entities.get_mut(&uid) {
            Some(e) => {
                *e.pos_mut() += *self.player_movement.lock() * /*make it slow this is player speed tick time magic const fixme*/0.04;
                self.conn.send(&ClientPacket::PlayerEntityUpdate{pos: *e.pos()}).expect("Could not send player position packet");
            },
            None => { entities.insert(uid, Entity::new(Vector3::new(0.0, 0.0, 0.0)));},
        }
    }

    pub fn alias<'a>(&'a self) -> MutexGuard<'a, String> {
        self.alias.lock()
    }

    pub fn conn<'a>(&'a self) -> &'a ClientConn {
        &self.conn
    }

    pub fn handle_packet(&self, packet: ServerPacket) {
        match packet {
            ServerPacket::Connected { player_entity_uid } => {
                *self.player_entity_uid.lock() = player_entity_uid;
                info!("Client connected");
            },
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

    pub fn start(client: Arc<Client>) {
        thread::spawn(move || {
            while client.running() {
                match client.conn().recv() {
                    Ok(data) => client.handle_packet(data.1),
                    Err(e) => warn!("Receive error: {:?}", e),
                }
                client.move_player();
            }
        });
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.conn.send(&ClientPacket::Disconnect).expect("Could not send disconnect packet");
    }
}

// // A thread-safe client handle
// pub struct ClientHandle {
//     client: Arc<Mutex<Client>>,
// }

// impl ClientHandle {
//     // Create a new client from a set of parameters and return a handle to it
//     pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: T, remote_addr: U) -> Result<ClientHandle, Error> {
//         Ok(ClientHandle {
//             client: Arc::new(Mutex::new(match client::Client::new(mode, alias, bind_addr, remote_addr) {
//                 Ok(c) => c,
//                 Err(e) => return Err(e),
//             })),
//         })
//     }

//     pub fn run(&mut self) {
//         let client_ref = self.client.clone();
//         thread::spawn(move || {
//             let conn = client_ref.lock().unwrap().conn();
//             while client_ref.lock().unwrap().running() {
//                 match conn.recv() {
//                     Ok(data) => client_ref.lock().unwrap().handle_packet(data.1),
//                     Err(e) => println!("[WARNING] Receive error: {:?}", e),
//                 }
//             }
//         });
//     }

//     pub fn set_chat_callback<F: 'static + Fn(&str, &str) + Send>(&self, f: F) {
//         self.client.lock().unwrap().set_chat_callback(f);
//     }

//     pub fn send_chat_msg(&self, msg: &str) -> Result<(), Error> {
//         self.client.lock().unwrap().send_chat_msg(msg)
//     }

//         pub fn send_command(&self, cmd: &str) -> Result<(), Error> {
//         self.client.lock().unwrap().send_command(cmd)
//     }
// }
