use bifrost::event::Event;
use bifrost::Relay;
use std::net::TcpStream;
use session::Session;
use common::network::packet::ClientPacket;
use world_context::World;
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
        println!("New session ! id: {}", self.session_id);
    }
}



pub struct PacketReceived {
    pub session_id: u32,
    pub data: ClientPacket,
}
impl Event<World> for PacketReceived {
    fn process(&self, relay: &Relay<World>, ctx: &mut World) {
        handle_packet(relay, ctx, self.session_id, &self.data);
    }
}
