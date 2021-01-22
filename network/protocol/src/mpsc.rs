use crate::{
    event::ProtocolEvent,
    frame::InitFrame,
    handshake::{ReliableDrain, ReliableSink},
    io::{UnreliableDrain, UnreliableSink},
    metrics::{ProtocolMetricCache, RemoveReason},
    types::Bandwidth,
    ProtocolError, RecvProtocol, SendProtocol,
};
use async_trait::async_trait;
use std::time::{Duration, Instant};

pub /* should be private */ enum MpscMsg {
    Event(ProtocolEvent),
    InitFrame(InitFrame),
}

#[derive(Debug)]
pub struct MpscSendProtcol<D>
where
    D: UnreliableDrain<DataFormat = MpscMsg>,
{
    drain: D,
    last: Instant,
    metrics: ProtocolMetricCache,
}

#[derive(Debug)]
pub struct MpscRecvProtcol<S>
where
    S: UnreliableSink<DataFormat = MpscMsg>,
{
    sink: S,
    metrics: ProtocolMetricCache,
}

impl<D> MpscSendProtcol<D>
where
    D: UnreliableDrain<DataFormat = MpscMsg>,
{
    pub fn new(drain: D, metrics: ProtocolMetricCache) -> Self {
        Self {
            drain,
            last: Instant::now(),
            metrics,
        }
    }
}

impl<S> MpscRecvProtcol<S>
where
    S: UnreliableSink<DataFormat = MpscMsg>,
{
    pub fn new(sink: S, metrics: ProtocolMetricCache) -> Self { Self { sink, metrics } }
}

#[async_trait]
impl<D> SendProtocol for MpscSendProtcol<D>
where
    D: UnreliableDrain<DataFormat = MpscMsg>,
{
    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError> {
        match &event {
            ProtocolEvent::Message {
                buffer,
                mid: _,
                sid,
            } => {
                let sid = *sid;
                let bytes = buffer.data.len() as u64;
                self.metrics.smsg_it(sid);
                self.metrics.smsg_ib(sid, bytes);
                let r = self.drain.send(MpscMsg::Event(event)).await;
                self.metrics.smsg_ot(sid, RemoveReason::Finished);
                self.metrics.smsg_ob(sid, RemoveReason::Finished, bytes);
                r
            },
            _ => self.drain.send(MpscMsg::Event(event)).await,
        }
    }

    async fn flush(&mut self, _: Bandwidth, _: Duration) -> Result<(), ProtocolError> { Ok(()) }
}

#[async_trait]
impl<S> RecvProtocol for MpscRecvProtcol<S>
where
    S: UnreliableSink<DataFormat = MpscMsg>,
{
    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError> {
        match self.sink.recv().await? {
            MpscMsg::Event(e) => {
                if let ProtocolEvent::Message {
                    buffer,
                    mid: _,
                    sid,
                } = &e
                {
                    let sid = *sid;
                    let bytes = buffer.data.len() as u64;
                    self.metrics.rmsg_it(sid);
                    self.metrics.rmsg_ib(sid, bytes);
                    self.metrics.rmsg_ot(sid, RemoveReason::Finished);
                    self.metrics.rmsg_ob(sid, RemoveReason::Finished, bytes);
                }
                Ok(e)
            },
            MpscMsg::InitFrame(_) => Err(ProtocolError::Closed),
        }
    }
}

#[async_trait]
impl<D> ReliableDrain for MpscSendProtcol<D>
where
    D: UnreliableDrain<DataFormat = MpscMsg>,
{
    async fn send(&mut self, frame: InitFrame) -> Result<(), ProtocolError> {
        self.drain.send(MpscMsg::InitFrame(frame)).await
    }
}

#[async_trait]
impl<S> ReliableSink for MpscRecvProtcol<S>
where
    S: UnreliableSink<DataFormat = MpscMsg>,
{
    async fn recv(&mut self) -> Result<InitFrame, ProtocolError> {
        match self.sink.recv().await? {
            MpscMsg::Event(_) => Err(ProtocolError::Closed),
            MpscMsg::InitFrame(f) => Ok(f),
        }
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use crate::{
        io::*,
        metrics::{ProtocolMetricCache, ProtocolMetrics},
    };
    use async_channel::*;
    use std::sync::Arc;

    pub struct ACDrain {
        sender: Sender<MpscMsg>,
    }

    pub struct ACSink {
        receiver: Receiver<MpscMsg>,
    }

    pub fn ac_bound(
        cap: usize,
        metrics: Option<ProtocolMetricCache>,
    ) -> [(MpscSendProtcol<ACDrain>, MpscRecvProtcol<ACSink>); 2] {
        let (s1, r1) = async_channel::bounded(cap);
        let (s2, r2) = async_channel::bounded(cap);
        let m = metrics.unwrap_or_else(|| {
            ProtocolMetricCache::new("mpsc", Arc::new(ProtocolMetrics::new().unwrap()))
        });
        [
            (
                MpscSendProtcol::new(ACDrain { sender: s1 }, m.clone()),
                MpscRecvProtcol::new(ACSink { receiver: r2 }, m.clone()),
            ),
            (
                MpscSendProtcol::new(ACDrain { sender: s2 }, m.clone()),
                MpscRecvProtcol::new(ACSink { receiver: r1 }, m.clone()),
            ),
        ]
    }

    #[async_trait]
    impl UnreliableDrain for ACDrain {
        type DataFormat = MpscMsg;

        async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError> {
            self.sender
                .send(data)
                .await
                .map_err(|_| ProtocolError::Closed)
        }
    }

    #[async_trait]
    impl UnreliableSink for ACSink {
        type DataFormat = MpscMsg;

        async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError> {
            self.receiver
                .recv()
                .await
                .map_err(|_| ProtocolError::Closed)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        mpsc::test_utils::*,
        types::{Pid, STREAM_ID_OFFSET1, STREAM_ID_OFFSET2},
        InitProtocol,
    };

    #[tokio::test]
    async fn handshake_all_good() {
        let [mut p1, mut p2] = ac_bound(10, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move { p2.initialize(false, Pid::fake(3), 42).await });
        let (r1, r2) = tokio::join!(r1, r2);
        assert_eq!(r1.unwrap(), Ok((Pid::fake(3), STREAM_ID_OFFSET1, 42)));
        assert_eq!(r2.unwrap(), Ok((Pid::fake(2), STREAM_ID_OFFSET2, 1337)));
    }
}
