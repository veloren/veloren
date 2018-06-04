use std::boxed::Box;
use std::net::ToSocketAddrs;

use network::client::ClientConn;
use network::packet::{ClientPacket, ServerPacket};

use ClientMode;
use Error;

pub struct Client {
    running: bool,
    conn: ClientConn,
    alias: String,
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
