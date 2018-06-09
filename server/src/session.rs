use std::net::TcpStream;
use common::Uid;
use std::thread::JoinHandle;
use std::thread;


use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt, NativeEndian, NetworkEndian};
use std::io::{Read, Write, Cursor};
use bifrost::relay::Relay;
use network::event::PacketReceived;
use world::World;
use common::network::packet::ServerPacket;
use player::Player;

pub struct Session {
    id: u32,
    listen_thread_handle: Option<JoinHandle<()>>,
    stream: TcpStream,
    player_id: Option<Uid>,
}

impl Session {
    pub fn new(id: u32, stream: TcpStream) -> Session {
        Session {
            id,
            listen_thread_handle: None,
            stream,
            player_id: None,
        }
    }

    pub fn start_listen_thread(&mut self, relay: Relay<World>) {

        let stream = self.stream.try_clone().unwrap();
        let session_id = self.id;

        self.listen_thread_handle = Some(thread::spawn(move || {
            Session::listen_for_packets(relay, session_id, stream);
        }));
    }

    fn listen_for_packets(relay : Relay<World>, session_id: u32, mut stream: TcpStream) {

        loop {

            let size = stream.read_u32::<LittleEndian>().unwrap() as usize;
            let mut data: Vec<u8> = Vec::with_capacity(size);
            data.resize(size, 0);
            stream.read_exact(data.as_mut()).unwrap();
            println!("Packet received ({:?}) : {:?}", &size, &data);
            relay.send(PacketReceived {
                session_id,
                data,
            });
        }

    }

    pub fn send_packet(&mut self, packet: &ServerPacket) {
        let data = packet.serialize().unwrap();
        self.stream.write_u32::<LittleEndian>(data.len() as u32);
        self.stream.write_all(&data);
        println!("Packet Send ({:?}) : {:?}", &data.len(), &data);

    }

    pub fn get_id(&self) -> u32 { self.id }

    pub fn set_player_id(&mut self, player_id: Option<Uid>) { self.player_id = player_id }
    pub fn get_player_id(&self) -> Option<Uid> { self.player_id }

    pub fn has_player(&self) -> bool { self.player_id.is_some() }
}
