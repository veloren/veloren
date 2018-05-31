extern crate network;

use std::net::ToSocketAddrs;
use network::client::ClientConn;
use network::packet::ClientPacket;

#[derive(Debug)]
pub enum Error {
    ConnectionErr,
}

pub enum ClientMode {
    Game,
    Headless,
}

pub struct Client {
    conn: ClientConn,
}

impl Client {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(mode: ClientMode, bind_addr: T, remote_addr: U) -> Result<Client, Error> {
        let conn = match ClientConn::new(bind_addr, remote_addr) {
            Ok(conn) => conn,
            Err(e) => panic!("ERR: {:?}", e), //return Err(Error::ConnectionErr),
        };

        Ok(Client {
            conn,
        })
    }

    pub fn connect(&mut self) -> bool {
        self.conn.send(ClientPacket::Connect{
            alias: "test-player".to_string(),
        })
    }

    pub fn send_chat_message(&mut self, msg: &str) -> bool {
        self.conn.send(ClientPacket::SendChatMsg{
            msg: msg.to_string(),
        })
    }
}
