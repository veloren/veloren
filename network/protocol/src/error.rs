/// All possible Errors that can happen during Handshake [`InitProtocol`]
///
/// [`InitProtocol`]: crate::InitProtocol
#[derive(Debug, PartialEq, Eq)]
pub enum InitProtocolError<E: std::fmt::Debug + Send> {
    Custom(E),
    /// expected Handshake, didn't get handshake
    NotHandshake,
    /// expected Id, didn't get id
    NotId,
    WrongMagicNumber([u8; 7]),
    WrongVersion([u32; 3]),
}

/// When you return closed you must stay closed!
#[derive(Debug, PartialEq, Eq)]
pub enum ProtocolError<E: std::fmt::Debug + Send> {
    /// Custom Error on the underlying I/O,
    /// e.g. the TCP, UDP or MPSC connection is dropped by the OS
    Custom(E),
    /// Violated indicates the veloren_network_protocol was violated
    /// the underlying I/O connection is still valid, but the remote side
    /// send WRONG (e.g. Invalid, or wrong order) data on the protocol layer.
    Violated,
}

impl<E: std::fmt::Debug + Send> From<ProtocolError<E>> for InitProtocolError<E> {
    fn from(err: ProtocolError<E>) -> Self {
        match err {
            ProtocolError::Custom(e) => InitProtocolError::Custom(e),
            ProtocolError::Violated => {
                unreachable!("not possible as the Init has raw access to the I/O")
            },
        }
    }
}

impl<E: std::fmt::Debug + Send> core::fmt::Display for InitProtocolError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InitProtocolError::Custom(e) => write!(f, "custom: {:?}", e),
            InitProtocolError::NotHandshake => write!(
                f,
                "Remote send something which couldn't be parsed as a handshake"
            ),
            InitProtocolError::NotId => {
                write!(f, "Remote send something which couldn't be parsed as an id")
            },
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

impl<E: std::fmt::Debug + Send> core::fmt::Display for ProtocolError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProtocolError::Custom(e) => write!(f, "Channel custom close: {:?}", e),
            ProtocolError::Violated => write!(f, "Channel protocol violated"),
        }
    }
}

impl<E: std::fmt::Debug + Send> std::error::Error for InitProtocolError<E> {}
impl<E: std::fmt::Debug + Send> std::error::Error for ProtocolError<E> {}
