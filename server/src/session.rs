use bifrost::Relay;
use common::network::packet_handler::PacketHandler;
use common::network::stream_helper::PacketSender;
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
use common::network::stream_helper::PacketReceiver;
use common::network::packet::ServerPacket;

pub struct Session {
    id: u32,
    listen_thread_handle: Option<JoinHandle<()>>,
    packet_sender: PacketSender,
    player_id: Option<Uid>,
}

impl Session {
    pub fn new(id: u32, stream: TcpStream) -> Session {
        Session {
            id,
            listen_thread_handle: None,
            packet_sender: PacketSender::new(stream),
            player_id: None,
        }
    }

    pub fn start_listen_thread(&mut self, relay: Relay<ServerContext>) {
        let packet_receiver = PacketReceiver::new(self.packet_sender.clone_stream());
        let id = self.id;
        self.listen_thread_handle = Some(thread::spawn(move || {
            Session::listen_for_packets(packet_receiver, relay, id);
        }));
    }

    fn listen_for_packets(mut packet_receiver: PacketReceiver, relay: Relay<ServerContext>, id: u32) {
        loop {
            match packet_receiver.recv_packet() {
                Ok(packet) => {
                    relay.send(PacketReceived {
                        session_id: id,
                        data: packet,
                    });
                },
                Err(_) => {
                    // TODO: Kick player
                },
            }
        }
    }

    pub fn send_packet(&self, packet: &ServerPacket) {
        match self.packet_sender.send_packet(packet) {
            Err(_) => {
                // TODO: Kick Player
            },
            _ => (),
        }
    }

    pub fn get_id(&self) -> u32 { self.id }

    pub fn set_player_id(&mut self, player_id: Option<Uid>) { self.player_id = player_id }
    pub fn get_player_id(&self) -> Option<Uid> { self.player_id }

    pub fn has_player(&self) -> bool { self.player_id.is_some() }
}
