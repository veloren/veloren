use std::io;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use local_ip;

use client::{ClientHandle, ClientMode};

pub struct GameData {
    pub client: ClientHandle,
}

pub struct GameHandle {
    game_data: Arc<Mutex<GameData>>,
}

impl GameHandle {
    pub fn new(alias: &str) -> GameHandle {
        let ip = local_ip::get().unwrap();

        // TODO: Seriously? This needs to go. Make it auto-detect this stuff
        // <rubbish>
        let mut port = String::new();
        println!("Local port [59001]:");
        io::stdin().read_line(&mut port).unwrap();
        let port = u16::from_str_radix(&port.trim(), 10).unwrap();

        let mut remote_addr = String::new();
        println!("Remote server address:");
        io::stdin().read_line(&mut remote_addr).unwrap();
        // </rubbish>

        GameHandle {
            game_data: Arc::new(Mutex::new(GameData {
                client: ClientHandle::new(ClientMode::Game, &alias, SocketAddr::new(ip, port), remote_addr)
                    .expect("Could not start client"),
            })),
        }
    }

    pub fn next_frame(&self) -> bool {
        // TODO
        true
    }
}
