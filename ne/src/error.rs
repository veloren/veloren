use crossbeam_channel::Receiver;

pub enum NetworkError {
    Io(std::io::Error),
    Serde(serde_cbor::Error),
    EngineShutdown,
}

pub type NetworkResult<T> = Result<T, NetworkError>;

impl From<std::io::Error> for NetworkError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_cbor::Error> for NetworkError {
    fn from(e: serde_cbor::Error) -> Self {
        Self::Serde(e)
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for NetworkError {
    fn from(_e: crossbeam_channel::SendError<T>) -> Self {
        Self::EngineShutdown
    }
}

impl From<crossbeam_channel::RecvError> for NetworkError {
    fn from(_e: crossbeam_channel::RecvError) -> Self {
        Self::EngineShutdown
    }
}

pub struct FutureNetworkResult<T> {
    done: bool,
    error_receiver: Receiver<NetworkResult<T>>,
}

impl<T> FutureNetworkResult<T> {
    pub(crate) fn new(error_receiver: Receiver<NetworkResult<T>>) -> Self {
        Self {
            done: false,
            error_receiver,
        }
    }

    pub(crate) fn now(result: NetworkResult<T>) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);
        tx.send(result).unwrap();
        Self::new(rx)
    }

    pub(crate) fn err_now(error: impl Into<NetworkError>) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);
        tx.send(Err(error.into())).unwrap();
        Self::new(rx)
    }

    pub fn poll(&mut self) -> Option<NetworkResult<T>> {
        if self.done {
            panic!("FutureNetworkError polled after receiving error");
        } else {
            self.done = true;
            self.error_receiver.try_recv().ok()
        }
    }
}
