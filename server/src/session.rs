use std::sync::Arc;
use network::event::PacketReceived;
use bifrost::Relay;
use std::net::TcpStream;
use common::Uid;
use std::thread::JoinHandle;
use std::thread;
use std::time;
use std::sync::MutexGuard;
use std::sync::Mutex;
use common::network::Error;
use common::network::packet_handler::PacketHandler;
use common::network::packet::{ClientPacket, ServerPacket};

use world::World;
use player::Player;

pub struct Session {
    id: u32,
    listen_thread_handle: Option<JoinHandle<()>>,
    handler: Arc<Mutex<PacketHandler>>,
    player_id: Option<Uid>,
}

impl Session {
    pub fn new(id: u32, stream: TcpStream) -> Session {
        Session {
            id,
            listen_thread_handle: None,
            handler: Arc::new(Mutex::new(PacketHandler::new(stream))),
            player_id: None,
        }
    }

    pub fn start_listen_thread(&mut self, relay: Relay<World>) {
        let handler_ref = self.handler.clone();
        let id = self.id;
        self.listen_thread_handle = Some(thread::spawn(move || {
            Session::listen_for_packets(handler_ref, relay, id);
        }));
    }

    fn listen_for_packets(handler: Arc<Mutex<PacketHandler>>, relay : Relay<World>, id: u32) {

        loop {
            match handler.lock().unwrap().recv_packet() {
                Ok(data) => {
                    relay.send(PacketReceived {
                        session_id: id,
                        data,
                    });
                },
                Err(e) => warn!("Receive error: {:?}", e),
            }
            // we need to sleep here, because otherwise we would almost always hold the lock on session.
            thread::sleep(time::Duration::from_millis(10));
        }

    }

    pub fn get_handler(&self) -> MutexGuard<PacketHandler> { self.handler.lock().unwrap() }

    pub fn get_id(&self) -> u32 { self.id }

    pub fn set_player_id(&mut self, player_id: Option<Uid>) { self.player_id = player_id }
    pub fn get_player_id(&self) -> Option<Uid> { self.player_id }

    pub fn has_player(&self) -> bool { self.player_id.is_some() }
}
