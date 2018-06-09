use bifrost::event::Event;
use world::World;
use std::net::TcpStream;
use bifrost::Relay;
use session::Session;
use common::network::packet::ClientPacket;
use network::handlers::handle_packet;

pub struct NewSessionEvent {
    pub session_id: u32,
    pub stream: TcpStream,
}
impl Event<World> for NewSessionEvent {
    fn process(&self, relay: &Relay<World>, ctx: &mut World) {
        let mut session = box Session::new(self.session_id, self.stream.try_clone().unwrap());
        session.start_listen_thread(relay.clone());
        ctx.add_session(session);
        println!("New session !");
    }
}



pub struct PacketReceived {
    pub session_id: u32,
    pub data: Vec<u8>,
}
impl Event<World> for PacketReceived {
    fn process(&self, relay: &Relay<World>, ctx: &mut World) {
        match ClientPacket::from(&self.data) {
            Ok(packet) => handle_packet(relay, ctx, self.session_id, &packet),
            Err(_) => println!("Cannot parse packet !"),
        }

    }
}


