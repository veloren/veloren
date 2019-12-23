use super::ClientState;
use crate::terrain::block::Block;
use crate::{comp, ChatType};
use vek::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Register {
        player: comp::Player,
        password: String,
    },
    Character {
        name: String,
        body: comp::Body,
        main: Option<String>, // Specifier for the weapon
    },
    ControllerInputs(comp::ControllerInputs),
    ControlEvent(comp::ControlEvent),
    RequestState(ClientState),
    SetViewDistance(u32),
    BreakBlock(Vec3<i32>),
    PlaceBlock(Vec3<i32>, Block),
    Ping,
    Pong,
    ChatMsg {
        chat_type: ChatType, // This is unused afaik, TODO: remove
        message: String,
    },
    PlayerPhysics {
        pos: comp::Pos,
        vel: comp::Vel,
        ori: comp::Ori,
    },
    TerrainChunkRequest {
        key: Vec2<i32>,
    },
    Disconnect,
}

impl ClientMsg {
    pub fn chat(message: String) -> ClientMsg {
        ClientMsg::ChatMsg {
            chat_type: ChatType::Chat,
            message,
        }
    }
    pub fn tell(message: String) -> ClientMsg {
        ClientMsg::ChatMsg {
            chat_type: ChatType::Tell,
            message,
        }
    }
    pub fn game(message: String) -> ClientMsg {
        ClientMsg::ChatMsg {
            chat_type: ChatType::GameUpdate,
            message,
        }
    }
    pub fn broadcast(message: String) -> ClientMsg {
        ClientMsg::ChatMsg {
            chat_type: ChatType::Broadcast,
            message,
        }
    }
    pub fn private(message: String) -> ClientMsg {
        ClientMsg::ChatMsg {
            chat_type: ChatType::Private,
            message,
        }
    }
    pub fn kill(message: String) -> ClientMsg {
        ClientMsg::ChatMsg {
            chat_type: ChatType::Private,
            message,
        }
    }
}
