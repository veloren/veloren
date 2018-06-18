// Library
use bifrost::{Relay, Event};

// Project
use common::net::{Conn, ClientPacket};

// Local
use session::Session;
use server_context::ServerContext;
use network::handlers::handle_packet;

pub struct NewSessionEvent {
    pub session_id: u32,
    pub conn: Conn,
}
impl Event<ServerContext> for NewSessionEvent {
    fn process(self: Box<Self>, relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
        let (send_conn, recv_conn) = self.conn.split();
        let mut session = box Session::new(self.session_id, send_conn);
        session.start_listen_thread(recv_conn, relay.clone());
        ctx.add_session(session);
        info!("New session ! id: {}", self.session_id);
    }
}

pub struct PacketReceived {
    pub session_id: u32,
    pub data: ClientPacket,
}
impl Event<ServerContext> for PacketReceived {
    fn process(self: Box<Self>, relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
        handle_packet(relay, ctx, self.session_id, self.data);
    }
}


pub struct KickSession {
    pub session_id: u32,
}
impl Event<ServerContext> for KickSession {
    fn process(self: Box<Self>, relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
        ctx.kick_session(self.session_id);
    }
}
