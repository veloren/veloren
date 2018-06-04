use std::boxed::Box;
use std::collections::HashMap;
use std::net::ToSocketAddrs;

use network::client::ClientConn;
use network::packet::{ClientPacket, ServerPacket};
use region::Entity;

use ClientMode;
use Error;

pub struct Client {
    running: bool,
    conn: ClientConn,
    alias: String,

    player_entity: Option<u64>, // TODO: Turn u64 into Uid
    entities: HashMap<u64, Entity>, // TODO: Turn u64 into Uid

    chat_callback: Box<Fn(&str, &str) + Send>,
}

impl Client {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: T, remote_addr: U) -> Result<Client, Error> {
        let conn = ClientConn::new(bind_addr, remote_addr)?;
        conn.send(&ClientPacket::Connect{ mode, alias: alias.to_string() })?;

        Ok(Client {
            running: true,
            conn,
            alias: alias.to_string(),

            player_entity: None,
            entities: HashMap::new(),

            chat_callback: Box::new(|_a, _s| {}),
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

    pub fn handle_packet(&mut self, packet: ServerPacket) {
        match packet {
            ServerPacket::Connected { player_entity } => self.player_entity = player_entity,
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

    pub fn send_chat_msg(&mut self, msg: &str) -> Result<(), Error> {
        self.conn.send(&ClientPacket::SendChatMsg{
            msg: msg.to_string(),
        })?;
        Ok(())
    }

    pub fn send_command(&mut self, cmd: &str) -> Result<(), Error> {
        self.conn.send(&ClientPacket::SendCommand{
            cmd: cmd.to_string(),
        })?;
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.conn.send(&ClientPacket::Disconnect).expect("Could not send disconnect packet");
    }
}
