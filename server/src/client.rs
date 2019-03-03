use specs::Entity as EcsEntity;
use common::{
    msg::{ServerMsg, ClientMsg},
    net::PostBox,
};

pub struct Client {
    pub ecs_entity: EcsEntity,
    pub postbox: PostBox<ServerMsg, ClientMsg>,
    pub last_ping: f64,
}
