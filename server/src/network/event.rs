use bifrost::{Relay, Event};
use std::net::TcpStream;
use session::Session;
use common::network::packet::ClientPacket;
use server_context::ServerContext;
use network::handlers::handle_packet;

pub struct NewSessionEvent {
    pub session_id: u32,
    pub stream: TcpStream,
}
impl Event<ServerContext> for NewSessionEvent {
    fn process(&self, relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
        let mut session = box Session::new(self.session_id, self.stream.try_clone().unwrap());
        session.start_listen_thread(relay.clone());
        ctx.add_session(session);
        info!("New session ! id: {}", self.session_id);
    }
}

pub struct PacketReceived {
    pub session_id: u32,
    pub data: ClientPacket,
}
impl Event<ServerContext> for PacketReceived {
    fn process(&self, relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
        handle_packet(relay, ctx, self.session_id, &self.data);
    }
}


pub struct KickSession {
    pub session_id: u32,
}
impl Event<ServerContext> for KickSession {
    fn process(&self, relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
        ctx.kick_session(self.session_id);
    }
}
