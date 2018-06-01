use std::boxed::Box;
use std::net::{SocketAddr, ToSocketAddrs};

use network::client::ClientConn;
use network::packet::{ClientPacket, ServerPacket};

use Error;

pub enum ClientMode {
    Game,
    Headless,
}

pub struct Client {
    running: bool,
    conn: ClientConn,
    alias: String,
    chat_callback: Box<Fn(&str, &str) + Send>,
}

impl Client {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: T, remote_addr: U) -> Result<Client, Error> {
        let conn = match ClientConn::new(bind_addr, remote_addr) {
            Ok(conn) => conn,
            Err(_) => return Err(Error::ConnectionErr),
        };

        if !conn.send(&ClientPacket::Connect{ alias: alias.to_string() }) {
            return Err(Error::ConnectionErr);
        }

        Ok(Client {
            running: true,
            conn,
            alias: alias.to_string(),
            chat_callback: Box::new(|_, _| {}),
        })
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn alias<'a>(&'a self) -> &'a str {
        &self.alias
    }

    pub fn conn(&self) -> ClientConn {
        self.conn.clone()
    }

    pub fn handle_packet(&mut self, data: (SocketAddr, ServerPacket)) {
        let (sock_addr, packet) = data;

        match packet {
            ServerPacket::Connected => {
                // Nothing yet
            },
            ServerPacket::Shutdown => self.running = false,
            ServerPacket::RecvChatMsg { alias, msg } => {
                (self.chat_callback)(&alias, &msg);
            },
            _ => {},
        }
    }

    pub fn set_chat_callback<F: 'static + Fn(&str, &str) + Send>(&mut self, f: F) {
        self.chat_callback = Box::new(f);
    }

    pub fn send_chat_msg(&mut self, msg: &str) -> bool {
        self.conn.send(&ClientPacket::SendChatMsg{
            msg: msg.to_string(),
        })
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.conn.send(&ClientPacket::Disconnect);
    }
}
