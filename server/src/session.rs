use bifrost::Relay;
use common::network::packet_handler::{PacketSender, PacketReceiver};
use common::Uid;
use network::event::PacketReceived;
use player::Player;
use server_context::ServerContext;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::thread::JoinHandle;
use std::time;
use common::network::packet::ServerPacket;
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
    packet_sender: RefCell<PacketSender>,
    player_id: Option<Uid>,
    state: Cell<SessionState>,
}

impl Session {
    pub fn new(id: u32, stream: TcpStream) -> Session {
        Session {
            id,
            listen_thread_handle: None,
            packet_sender: RefCell::new(PacketSender::new(Some(stream), None)),
            player_id: None,
            state: Cell::new(SessionState::Connected),
        }
    }

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

    pub fn send_packet(&self, packet: &ServerPacket) {
        match self.packet_sender.borrow_mut().send_packet(packet) {
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
