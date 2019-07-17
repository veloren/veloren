use super::ClientState;
use crate::terrain::block::Block;
use crate::{comp, ChatType};
use vek::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Register {
        player: comp::Player,
    },
    Character {
        name: String,
        body: comp::Body,
    },
    Controller(comp::Controller),
    RequestState(ClientState),
    SetViewDistance(u32),
    BreakBlock(Vec3<i32>),
    PlaceBlock(Vec3<i32>, Block),
    Ping,
    Pong,
    ChatMsg {
        chat_type: ChatType,
        msg: String,
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
    pub fn chat(message: String) -> crate::msg::client::ClientMsg {
        crate::msg::client::ClientMsg::ChatMsg {
            chat_type: ChatType::Chat,
            msg: message,
        }
    }
    pub fn tell(message: String) -> crate::msg::client::ClientMsg {
        crate::msg::client::ClientMsg::ChatMsg {
            chat_type: ChatType::Tell,
            msg: message,
        }
    }
}
