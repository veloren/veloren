extern crate network;

use network::client::ClientConn;

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
    pub fn new(mode: ClientMode, bind_addr: &str, remote_addr: &str) -> Result<Client, Error> {
        let conn = match ClientConn::new(bind_addr, remote_addr) {
            Ok(conn) => conn,
            Err(_) => return Err(Error::ConnectionErr),
        };

        Ok(Client {
            conn,
        })
    }
}
