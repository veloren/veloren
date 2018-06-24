// Standard
use std::net::TcpStream;
use std::sync::{Arc, Mutex, MutexGuard};
use std::cell::{RefCell, Cell};
use std::thread;
use std::thread::JoinHandle;
use std::time;

// Library
use bifrost::Relay;

// Project
use common::net::{Connection, ServerMessage, ClientMessage, Message, UdpMgr};
use common::Uid;

// Local
use network::event::{PacketReceived, KickSession};
use player::Player;
use server_context::ServerContext;

#[derive(Copy, Clone, PartialEq)]
pub enum SessionState {
    Connected,
    ShouldKick,
}

pub struct Session {
    id: u32,
    listen_thread_handle: Option<JoinHandle<()>>,
    conn: Arc<Connection<ClientMessage>>,
    player_id: Option<Uid>,
    state: Cell<SessionState>,
}

impl Session {
    pub fn new(id: u32, stream: TcpStream, udpmgr: Arc<UdpMgr>, relay: &Relay<ServerContext>) -> Session {
        let relay = relay.clone();
        let conn = Connection::new_stream(stream, Box::new(move |m| {
            //callback message
            let ret = *m;
            match ret {
                Ok(message) => {
                    relay.send(PacketReceived {
                        session_id: id,
                        data: message,
                    });
                },
                _ => {
                    relay.send(KickSession { session_id: id });
                },
            };
        }), None, udpmgr).unwrap();
        Connection::start(&conn);
        let session = Session {
            id,
            listen_thread_handle: None,
            conn,
            player_id: None,
            state: Cell::new(SessionState::Connected),
        };

        return session;
    }

    pub fn send_message(&self, message: ServerMessage) {
        self.conn.send(message);
        /*
        match self.packet_sender.borrow_mut().send_packet(packet) {
            Ok(_) => {},
            Err(_) => self.state.set(SessionState::ShouldKick),
        }*/
    }

    pub fn stop_conn(&self) {
        Connection::stop(&self.conn);
    }

    pub fn get_id(&self) -> u32 { self.id }

    pub fn set_player_id(&mut self, player_id: Option<Uid>) { self.player_id = player_id }
    pub fn get_player_id(&self) -> Option<Uid> { self.player_id }

    pub fn has_player(&self) -> bool { self.player_id.is_some() }

    pub fn should_kick(&self) -> bool { self.state.get() == SessionState::ShouldKick }
}
