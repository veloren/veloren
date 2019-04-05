#[derive(Debug)]
pub enum PostError {
    InvalidMessage,
    InternalError,
    Disconnected,
}

#[derive(Debug)]
pub enum PostErrorInternal {
    Io(std::io::Error),
    Serde(bincode::Error),
    ChannelRecv(std::sync::mpsc::TryRecvError),
    ChannelSend, // Empty because I couldn't figure out how to handle generic type in mpsc::TrySendError properly
    MsgSizeLimitExceeded,
    MioError,
}

impl<'a, T: Into<&'a PostErrorInternal>> From<T> for PostError {
    fn from(err: T) -> Self {
        match err.into() {
            // TODO: Are I/O errors always disconnect errors?
            PostErrorInternal::Io(_) => PostError::Disconnected,
            PostErrorInternal::Serde(_) => PostError::InvalidMessage,
            PostErrorInternal::MsgSizeLimitExceeded => PostError::InvalidMessage,
            PostErrorInternal::MioError => PostError::InternalError,
            PostErrorInternal::ChannelRecv(_) => PostError::InternalError,
            PostErrorInternal::ChannelSend => PostError::InternalError,
        }
    }
}

impl From<PostErrorInternal> for PostError {
    fn from(err: PostErrorInternal) -> Self {
        (&err).into()
    }
}

impl From<std::io::Error> for PostErrorInternal {
    fn from(err: std::io::Error) -> Self {
        PostErrorInternal::Io(err)
    }
}

impl From<bincode::Error> for PostErrorInternal {
    fn from(err: bincode::Error) -> Self {
        PostErrorInternal::Serde(err)
    }
}

impl From<std::sync::mpsc::TryRecvError> for PostErrorInternal {
    fn from(err: std::sync::mpsc::TryRecvError) -> Self {
        PostErrorInternal::ChannelRecv(err)
    }
}



impl From<std::io::Error> for PostError {
    fn from(err: std::io::Error) -> Self {
        (&PostErrorInternal::from(err)).into()
    }
}

impl From<bincode::Error> for PostError {
    fn from(err: bincode::Error) -> Self {
        (&PostErrorInternal::from(err)).into()
    }
}

impl From<std::sync::mpsc::TryRecvError> for PostError {
    fn from(err: std::sync::mpsc::TryRecvError) -> Self {
        (&PostErrorInternal::from(err)).into()
    }
}
