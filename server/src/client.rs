use common::{
    msg::{ClientMsg, ClientState, RequestStateError, ServerMsg},
    net::PostBox,
};
use hashbrown::HashMap;
use specs::Entity as EcsEntity;

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
        self.postbox
            .send_message(ServerMsg::StateAnswer(Ok(new_state)));
    }
    pub fn error_state(&mut self, error: RequestStateError) {
        self.postbox
            .send_message(ServerMsg::StateAnswer(Err((error, self.client_state))));
    }
    pub fn force_state(&mut self, new_state: ClientState) {
        self.client_state = new_state;
        self.postbox.send_message(ServerMsg::ForceState(new_state));
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

    pub fn len(&mut self) -> usize {
        self.clients.len()
    }

    pub fn add(&mut self, entity: EcsEntity, client: Client) {
        self.clients.insert(entity, client);
    }

    pub fn get<'a>(&'a self, entity: &EcsEntity) -> Option<&'a Client> {
        self.clients.get(entity)
    }

    pub fn get_mut<'a>(&'a mut self, entity: &EcsEntity) -> Option<&'a mut Client> {
        self.clients.get_mut(entity)
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
            if client.client_state == ClientState::Spectator
                || client.client_state == ClientState::Character
                || client.client_state == ClientState::Dead
            {
                client.notify(msg.clone());
            }
        }
    }

    pub fn notify_ingame_if<F: FnMut(EcsEntity) -> bool>(&mut self, msg: ServerMsg, mut f: F) {
        for (_entity, client) in self.clients.iter_mut().filter(|(e, _)| f(**e)) {
            if client.client_state == ClientState::Spectator
                || client.client_state == ClientState::Character
                || client.client_state == ClientState::Dead
            {
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
            if (client.client_state == ClientState::Spectator
                || client.client_state == ClientState::Character
                || client.client_state == ClientState::Dead)
                && *entity != except_entity
            {
                client.notify(msg.clone());
            }
        }
    }

    pub fn notify_ingame_if_except<F: FnMut(EcsEntity) -> bool>(
        &mut self,
        except_entity: EcsEntity,
        msg: ServerMsg,
        mut f: F,
    ) {
        for (entity, client) in self.clients.iter_mut().filter(|(e, _)| f(**e)) {
            if (client.client_state == ClientState::Spectator
                || client.client_state == ClientState::Character
                || client.client_state == ClientState::Dead)
                && *entity != except_entity
            {
                client.notify(msg.clone());
            }
        }
    }
}
