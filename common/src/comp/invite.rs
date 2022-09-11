use serde::{Deserialize, Serialize};
use specs::Component;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InviteKind {
    Group,
    Trade,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InviteResponse {
    Accept,
    Decline,
}

pub struct Invite {
    pub inviter: specs::Entity,
    pub kind: InviteKind,
}

impl Component for Invite {
    type Storage = specs::DenseVecStorage<Self>;
}

/// Pending invites that an entity currently has sent out
/// (invited entity, instant when invite times out)
pub struct PendingInvites(pub Vec<(specs::Entity, InviteKind, std::time::Instant)>);
impl Component for PendingInvites {
    type Storage = specs::DenseVecStorage<Self>;
}
