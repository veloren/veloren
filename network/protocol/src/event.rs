use crate::{
    frame::OTFrame,
    types::{Bandwidth, Prio, Promises, Sid},
};
use bytes::Bytes;

/// used for communication with [`SendProtocol`] and [`RecvProtocol`]
///
/// [`SendProtocol`]: crate::SendProtocol
/// [`RecvProtocol`]: crate::RecvProtocol
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum ProtocolEvent {
    Shutdown,
    OpenStream {
        sid: Sid,
        prio: Prio,
        promises: Promises,
        guaranteed_bandwidth: Bandwidth,
    },
    CloseStream {
        sid: Sid,
    },
    Message {
        data: Bytes,
        sid: Sid,
    },
}

impl ProtocolEvent {
    pub(crate) fn to_frame(&self) -> OTFrame {
        match self {
            ProtocolEvent::Shutdown => OTFrame::Shutdown,
            ProtocolEvent::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth,
            } => OTFrame::OpenStream {
                sid: *sid,
                prio: *prio,
                promises: *promises,
                guaranteed_bandwidth: *guaranteed_bandwidth,
            },
            ProtocolEvent::CloseStream { sid } => OTFrame::CloseStream { sid: *sid },
            ProtocolEvent::Message { .. } => {
                unimplemented!("Event::Message to OTFrame IS NOT supported")
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_frame() {
        assert_eq!(ProtocolEvent::Shutdown.to_frame(), OTFrame::Shutdown);
        assert_eq!(
            ProtocolEvent::CloseStream { sid: Sid::new(42) }.to_frame(),
            OTFrame::CloseStream { sid: Sid::new(42) }
        );
    }

    #[test]
    #[should_panic]
    fn test_msg_buffer_panic() {
        let _ = ProtocolEvent::Message {
            data: Bytes::new(),
            sid: Sid::new(23),
        }
        .to_frame();
    }
}
