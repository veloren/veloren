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
use common::net::{Conn, SendConn, RecvConn, ServerPacket};
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
    send_conn: SendConn,
    player_id: Option<Uid>,
    state: Cell<SessionState>,
}

impl Session {
    pub fn new(id: u32, send_conn: SendConn) -> Session {
        Session {
            id,
            listen_thread_handle: None,
            send_conn,
            player_id: None,
            state: Cell::new(SessionState::Connected),
        }
    }

    pub fn start_listen_thread(&mut self, recv_conn: RecvConn, relay: Relay<ServerContext>) {
        let id = self.id;
        self.listen_thread_handle = Some(thread::spawn(move || {
            Session::listen_for_packets(recv_conn, relay, id);
        }));
    }

    fn listen_for_packets(recv_conn: RecvConn, relay: Relay<ServerContext>, session_id: u32) {
        loop {
            match recv_conn.recv() {
                Ok(packet) => {
                    relay.send(PacketReceived {
                        session_id,
                        data: packet,
                    });
                },
                _ => {
                    relay.send(KickSession { session_id });
                    break;
                },
            }
        }
    }

    pub fn send_packet(&self, packet: &ServerPacket) {
        match self.send_conn.send(packet) {
            Ok(_) => {},
            Err(_) => self.state.set(SessionState::ShouldKick),
        }
    }

    pub fn get_id(&self) -> u32 { self.id }

    pub fn set_player_id(&mut self, player_id: Option<Uid>) { self.player_id = player_id }
    pub fn get_player_id(&self) -> Option<Uid> { self.player_id }

    pub fn has_player(&self) -> bool { self.player_id.is_some() }

    pub fn should_kick(&self) -> bool { self.state.get() == SessionState::ShouldKick }
}
