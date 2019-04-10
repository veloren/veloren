use std::collections::HashMap;
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
    pub postbox: PostBox<ServerMsg, ClientMsg>,
    pub last_ping: f64,
}

impl Client {
    pub fn notify(&mut self, msg: ServerMsg) {
        self.postbox.send(msg);
    }
}

pub struct Clients {
    clients: HashMap<EcsEntity, Client>,
}

impl Clients {
    pub fn empty() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn add(&mut self, entity: EcsEntity, client: Client) {
        self.clients.insert(entity, client);
    }

    pub fn remove_if<F: FnMut(EcsEntity, &mut Client) -> bool>(&mut self, mut f: F) {
        self.clients.retain(|entity, client| !f(*entity, client));
    }

    pub fn notify(&mut self, entity: EcsEntity, msg: ServerMsg) {
        if let Some(client) = self.clients.get_mut(&entity) {
            client.notify(msg);
        }
    }

    pub fn notify_connected(&mut self, msg: ServerMsg) {
        for client in self.clients.values_mut() {
            if client.state == ClientState::Connected {
                client.notify(msg.clone());
            }
        }
    }

    pub fn notify_connected_except(&mut self, except_entity: EcsEntity, msg: ServerMsg) {
        for (entity, client) in self.clients.iter_mut() {
            if client.state == ClientState::Connected && *entity != except_entity {
                client.notify(msg.clone());
            }
        }
    }
}
