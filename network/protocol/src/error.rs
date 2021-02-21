/// All possible Errors that can happen during Handshake [`InitProtocol`]
///
/// [`InitProtocol`]: crate::InitProtocol
#[derive(Debug, PartialEq)]
pub enum InitProtocolError {
    Closed,
    WrongMagicNumber([u8; 7]),
    WrongVersion([u32; 3]),
}

/// When you return closed you must stay closed!
#[derive(Debug, PartialEq)]
pub enum ProtocolError {
    Closed,
}

impl From<ProtocolError> for InitProtocolError {
    fn from(err: ProtocolError) -> Self {
        match err {
            ProtocolError::Closed => InitProtocolError::Closed,
        }
    }
}

impl core::fmt::Display for InitProtocolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InitProtocolError::Closed => write!(f, "Channel closed"),
            InitProtocolError::WrongMagicNumber(r) => write!(
                f,
                "Magic Number doesn't match, remote side send '{:?}' instead of '{:?}'",
                &r,
                &crate::types::VELOREN_MAGIC_NUMBER
            ),
            InitProtocolError::WrongVersion(r) => write!(
                f,
                "Network doesn't match, remote side send '{:?}' we are on '{:?}'",
                &r,
                &crate::types::VELOREN_NETWORK_VERSION
            ),
        }
    }
}

impl core::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProtocolError::Closed => write!(f, "Channel closed"),
        }
    }
}

impl std::error::Error for InitProtocolError {}
impl std::error::Error for ProtocolError {}
