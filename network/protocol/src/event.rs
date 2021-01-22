use crate::{
    frame::Frame,
    message::MessageBuffer,
    types::{Bandwidth, Mid, Prio, Promises, Sid},
};
use std::sync::Arc;

/* used for communication with Protocols */
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
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
        buffer: Arc<MessageBuffer>,
        mid: Mid,
        sid: Sid,
    },
}

impl ProtocolEvent {
    pub(crate) fn to_frame(&self) -> Frame {
        match self {
            ProtocolEvent::Shutdown => Frame::Shutdown,
            ProtocolEvent::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth: _,
            } => Frame::OpenStream {
                sid: *sid,
                prio: *prio,
                promises: *promises,
            },
            ProtocolEvent::CloseStream { sid } => Frame::CloseStream { sid: *sid },
            ProtocolEvent::Message { .. } => {
                unimplemented!("Event::Message to Frame IS NOT supported")
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_frame() {
        assert_eq!(ProtocolEvent::Shutdown.to_frame(), Frame::Shutdown);
        assert_eq!(
            ProtocolEvent::CloseStream { sid: Sid::new(42) }.to_frame(),
            Frame::CloseStream { sid: Sid::new(42) }
        );
    }

    #[test]
    #[should_panic]
    fn test_sixlet_to_str() {
        let _ = ProtocolEvent::Message {
            buffer: Arc::new(MessageBuffer { data: vec![] }),
            mid: 0,
            sid: Sid::new(23),
        }
        .to_frame();
    }
}
