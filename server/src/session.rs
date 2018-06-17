use network::event::PacketReceived;
use std::net::ToSocketAddrs;
use bifrost::Relay;
use common::net::{Connection, ServerMessage, ClientMessage, Message};
use common::Uid;
use player::Player;
use server_context::ServerContext;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::thread::JoinHandle;
use std::time;
use std::cell::RefCell;
use std::cell::Cell;
use network::event::KickSession;

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
    pub fn new(id: u32, stream: TcpStream, relay: &Relay<ServerContext>) -> Session {
        let relay = relay.clone();
        let conn = Connection::new_stream(stream, Box::new(move |m| {
            //callback message
            relay.send(PacketReceived {
                session_id: id,
                data: *m,
            });
        }), None).unwrap();
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
/*
    pub fn start_listen_thread(&mut self, relay: Relay<ServerContext>) {
        let packet_receiver = PacketReceiver::new(Some(self.packet_sender.borrow_mut().clone_tcp_stream()), None);
        let id = self.id;
        self.listen_thread_handle = Some(thread::spawn(move || {
            Session::listen_for_packets(packet_receiver, relay, id);
        }));
    }

    fn listen_for_packets(mut packet_receiver: PacketReceiver, relay: Relay<ServerContext>, session_id: u32) {
        loop {
            match packet_receiver.recv_packet() {
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
*/
    pub fn send_message(&self, message: ServerMessage) {
        self.conn.send(message);
        /*
        match self.packet_sender.borrow_mut().send_packet(packet) {
            Ok(_) => {},
            Err(_) => self.state.set(SessionState::ShouldKick),
        }*/
    }

    pub fn get_id(&self) -> u32 { self.id }

    pub fn set_player_id(&mut self, player_id: Option<Uid>) { self.player_id = player_id }
    pub fn get_player_id(&self) -> Option<Uid> { self.player_id }

    pub fn has_player(&self) -> bool { self.player_id.is_some() }

    pub fn should_kick(&self) -> bool { self.state.get() == SessionState::ShouldKick }
}
