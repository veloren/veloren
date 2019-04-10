use specs::Entity as EcsEntity;
use common::{
    comp,
    msg::{ServerMsg, ClientMsg},
    net::PostBox,
};
use crate::Error;

#[derive(PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
}

pub struct Client {
    pub state: ClientState,
    pub entity: EcsEntity,
    pub postbox: PostBox<ServerMsg, ClientMsg>,
    pub last_ping: f64,
}

impl Client {
    pub fn notify(&mut self, msg: ServerMsg) {
        self.postbox.send(msg);
    }
}

pub struct Clients {
    clients: Vec<Client>,
}

impl Clients {
    pub fn empty() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    pub fn add(&mut self, client: Client) {
        self.clients.push(client);
    }

    pub fn remove_if<F: FnMut(&mut Client) -> bool>(&mut self, f: F) {
        self.clients.drain_filter(f);
    }

    pub fn notify_connected(&mut self, msg: ServerMsg) {
        for client in &mut self.clients {
            if client.state == ClientState::Connected {
                client.postbox.send(msg.clone());
            }
        }
    }

    pub fn notify_connected_except(&mut self, entity: EcsEntity, msg: ServerMsg) {
        for client in &mut self.clients {
            if client.entity != entity && client.state == ClientState::Connected {
                client.postbox.send(msg.clone());
            }
        }
    }
}
