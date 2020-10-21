use common::msg::{ClientGeneral, ClientRegister, PingMsg, ServerGeneral, ServerRegisterAnswer};

use network::{Message, Stream, StreamError};
use serde::{de::DeserializeOwned, Serialize};

use specs::Component;
use specs_idvs::IdvStorage;

/// helped to reduce code duplication
pub(crate) trait GetStream {
    type RecvMsg: DeserializeOwned;
    type SendMsg: Serialize + core::fmt::Debug;
    fn get_mut(&mut self) -> &mut Stream;
    fn verify(msg: &Self::SendMsg) -> bool;

    fn send(&mut self, msg: Self::SendMsg) -> Result<(), StreamError> {
        if Self::verify(&msg) {
            self.get_mut().send(msg)
        } else {
            unreachable!("sending this msg isn't allowed! got: {:?}", msg)
        }
    }

    fn send_fallible(&mut self, msg: Self::SendMsg) { let _ = self.send(msg); }

    fn prepare(&mut self, msg: &Self::SendMsg) -> Message {
        if Self::verify(&msg) {
            Message::serialize(&msg, &self.get_mut())
        } else {
            unreachable!("sending this msg isn't allowed! got: {:?}", msg)
        }
    }
}

// Streams
// we ignore errors on send, and do unified error handling in recv
pub struct GeneralStream(pub(crate) Stream);
pub struct PingStream(pub(crate) Stream);
pub struct RegisterStream(pub(crate) Stream);
pub struct CharacterScreenStream(pub(crate) Stream);
pub struct InGameStream(pub(crate) Stream);

impl Component for GeneralStream {
    type Storage = IdvStorage<Self>;
}
impl Component for PingStream {
    type Storage = IdvStorage<Self>;
}
impl Component for RegisterStream {
    type Storage = IdvStorage<Self>;
}
impl Component for CharacterScreenStream {
    type Storage = IdvStorage<Self>;
}
impl Component for InGameStream {
    type Storage = IdvStorage<Self>;
}

impl GetStream for GeneralStream {
    type RecvMsg = ClientGeneral;
    type SendMsg = ServerGeneral;

    fn get_mut(&mut self) -> &mut Stream { &mut self.0 }

    fn verify(msg: &Self::SendMsg) -> bool {
        matches!(&msg, ServerGeneral::PlayerListUpdate(_)
            | ServerGeneral::ChatMsg(_)
            | ServerGeneral::SetPlayerEntity(_)
            | ServerGeneral::TimeOfDay(_)
            | ServerGeneral::EntitySync(_)
            | ServerGeneral::CompSync(_)
            | ServerGeneral::CreateEntity(_)
            | ServerGeneral::DeleteEntity(_)
            | ServerGeneral::Disconnect(_)
            | ServerGeneral::Notification(_))
    }
}
impl GetStream for PingStream {
    type RecvMsg = PingMsg;
    type SendMsg = PingMsg;

    fn get_mut(&mut self) -> &mut Stream { &mut self.0 }

    fn verify(_: &Self::SendMsg) -> bool { true }
}
impl GetStream for RegisterStream {
    type RecvMsg = ClientRegister;
    type SendMsg = ServerRegisterAnswer;

    fn get_mut(&mut self) -> &mut Stream { &mut self.0 }

    fn verify(_: &Self::SendMsg) -> bool { true }
}
impl GetStream for CharacterScreenStream {
    type RecvMsg = ClientGeneral;
    type SendMsg = ServerGeneral;

    fn get_mut(&mut self) -> &mut Stream { &mut self.0 }

    fn verify(msg: &Self::SendMsg) -> bool {
        matches!(&msg, ServerGeneral::CharacterDataLoadError(_)
            | ServerGeneral::CharacterListUpdate(_)
            | ServerGeneral::CharacterActionError(_)
            | ServerGeneral::CharacterSuccess)
    }
}
impl GetStream for InGameStream {
    type RecvMsg = ClientGeneral;
    type SendMsg = ServerGeneral;

    fn get_mut(&mut self) -> &mut Stream { &mut self.0 }

    fn verify(msg: &Self::SendMsg) -> bool {
        matches!(&msg, ServerGeneral::GroupUpdate(_)
            | ServerGeneral::GroupInvite { .. }
            | ServerGeneral::InvitePending(_)
            | ServerGeneral::InviteComplete { .. }
            | ServerGeneral::ExitInGameSuccess
            | ServerGeneral::InventoryUpdate(_, _)
            | ServerGeneral::TerrainChunkUpdate { .. }
            | ServerGeneral::TerrainBlockUpdates(_)
            | ServerGeneral::SetViewDistance(_)
            | ServerGeneral::Outcomes(_)
            | ServerGeneral::Knockback(_))
    }
}
