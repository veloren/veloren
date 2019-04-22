use std::collections::HashMap;
use specs::Entity as EcsEntity;
use common::{
    comp,
    msg::{ServerMsg, ClientMsg, ClientState, RequestStateError},
    net::PostBox,
};
use crate::Error;

pub struct Client {
    pub client_state: ClientState,
    pub postbox: PostBox<ServerMsg, ClientMsg>,
    pub last_ping: f64,
}

impl Client {
    pub fn notify(&mut self, msg: ServerMsg) {
        self.postbox.send_message(msg);
    }
    pub fn allow_state(&mut self, new_state: ClientState) {
        self.client_state = new_state;
        self.postbox.send_message(ServerMsg::StateAnswer(
                Ok(new_state)));
    }
    pub fn error_state(&mut self, error: RequestStateError) {
        self.postbox.send_message(ServerMsg::StateAnswer(
                Err((error, self.client_state))));
    }
    pub fn force_state(&mut self, new_state: ClientState) {
        self.client_state = new_state;
        self.postbox.send_message(ServerMsg::ForceState(
                new_state));
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

    pub fn notify_registered(&mut self, msg: ServerMsg) {
        for client in self.clients.values_mut() {
            if client.client_state != ClientState::Connected {
                client.notify(msg.clone());
            }
        }
    }

    pub fn notify_ingame(&mut self, msg: ServerMsg) {
        for client in self.clients.values_mut() {
            if client.client_state == ClientState::Spectator || client.client_state == ClientState::Character {
                client.notify(msg.clone());
            }
        }
    }

    pub fn notify_registered_except(&mut self, except_entity: EcsEntity, msg: ServerMsg) {
        for (entity, client) in self.clients.iter_mut() {
            if client.client_state != ClientState::Connected && *entity != except_entity {
                client.notify(msg.clone());
            }
        }
    }

    pub fn notify_ingame_except(&mut self, except_entity: EcsEntity, msg: ServerMsg) {
        for (entity, client) in self.clients.iter_mut() {
            if (client.client_state == ClientState::Spectator || client.client_state == ClientState::Character)
                && *entity != except_entity {
                client.notify(msg.clone());
            }
        }
    }
}
