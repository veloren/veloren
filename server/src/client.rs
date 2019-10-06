use common::{
    msg::{ClientMsg, ClientState, RequestStateError, ServerMsg},
    net::PostBox,
};
use hashbrown::{hash_map::DefaultHashBuilder, HashSet};
use indexmap::IndexMap;
use specs::Entity as EcsEntity;
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use vek::*;

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
    clients: IndexMap<EcsEntity, Client, DefaultHashBuilder>,
}

impl Clients {
    pub fn empty() -> Self {
        Self {
            clients: IndexMap::default(),
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
        self.clients.get_mut(entit:y)
    }

    pub fn remove<'a>(&'a mut self, entity: &EcsEntity) -> Option<Client> {
        self.clients.remove(entity)
    }

    pub fn get_client_index_ingame<'a>(&'a mut self, entity: &EcsEntity) -> Option<usize> {
        self.clients.get_full(entity).and_then(|(i, _, c)| {
            if c.client_state == ClientState::Spectator
                || c.client_state == ClientState::Character
                || c.client_state == ClientState::Dead
            {
                Some(i)
            } else {
                None
            }
        })
    }

    //pub fn get_index_mut<'a>(&'a mut self, index: u32) -> Option<&'a mut Client> {
    //    self.clients.get_index_mut(index)
    //}

    pub fn remove_if<F: FnMut(EcsEntity, &mut Client) -> bool>(&mut self, mut f: F) {
        self.clients.retain(|entity, client| !f(*entity, client));
    }

    pub fn notify_index(&mut self, index: usize, msg: ServerMsg) {
        if let Some((_, client)) = self.clients.get_index_mut(index) {
            client.notify(msg);
        }
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

// Distance from fuzzy_chunk before snapping to current chunk
pub const CHUNK_FUZZ: u32 = 2;
// Distance out of the range of a region before removing it from subscriptions
pub const REGION_FUZZ: u32 = 16;

#[derive(Clone, Debug)]
pub struct RegionSubscription {
    pub fuzzy_chunk: Vec2<i32>,
    pub regions: HashSet<Vec2<i32>>,
}

impl Component for RegionSubscription {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
