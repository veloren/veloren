#[cfg(feature = "metrics")]
use crate::metrics::RemoveReason;
use crate::{
    error::ProtocolError,
    event::ProtocolEvent,
    frame::InitFrame,
    handshake::{ReliableDrain, ReliableSink},
    metrics::ProtocolMetricCache,
    types::{Bandwidth, Promises},
    RecvProtocol, SendProtocol, UnreliableDrain, UnreliableSink,
};
use async_trait::async_trait;
use std::time::{Duration, Instant};
#[cfg(feature = "trace_pedantic")]
use tracing::trace;

/// used for implementing your own MPSC `Sink` and `Drain`
#[derive(Debug)]
pub enum MpscMsg {
    Event(ProtocolEvent),
    InitFrame(InitFrame),
}

/// MPSC implementation of [`SendProtocol`]
///
/// [`SendProtocol`]: crate::SendProtocol
#[derive(Debug)]
pub struct MpscSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = MpscMsg>,
{
    drain: D,
    #[allow(dead_code)]
    last: Instant,
    metrics: ProtocolMetricCache,
}

/// MPSC implementation of [`RecvProtocol`]
///
/// [`RecvProtocol`]: crate::RecvProtocol
#[derive(Debug)]
pub struct MpscRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = MpscMsg>,
{
    sink: S,
    metrics: ProtocolMetricCache,
}

impl<D> MpscSendProtocol<D>
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

    /// returns all promises that this Protocol can take care of
    /// If you open a Stream anyway, unsupported promises are ignored.
    pub fn supported_promises() -> Promises {
        Promises::ORDERED
            | Promises::CONSISTENCY
            | Promises::GUARANTEED_DELIVERY
            | Promises::COMPRESSED
            | Promises::ENCRYPTED /*assume a direct mpsc connection is secure*/
    }
}

impl<S> MpscRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = MpscMsg>,
{
    pub fn new(sink: S, metrics: ProtocolMetricCache) -> Self { Self { sink, metrics } }
}

#[async_trait]
impl<D> SendProtocol for MpscSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = MpscMsg>,
{
    type CustomErr = D::CustomErr;

    fn notify_from_recv(&mut self, _event: ProtocolEvent) {}

    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError<Self::CustomErr>> {
        #[cfg(feature = "trace_pedantic")]
        trace!(?event, "send");
        match &event {
            ProtocolEvent::Message {
                data: _data,
                sid: _sid,
            } => {
                #[cfg(feature = "metrics")]
                let (bytes, line) = {
                    let sid = *_sid;
                    let bytes = _data.len() as u64;
                    let line = self.metrics.init_sid(sid);
                    line.smsg_it.inc();
                    line.smsg_ib.inc_by(bytes);
                    (bytes, line)
                };
                let r = self.drain.send(MpscMsg::Event(event)).await;
                #[cfg(feature = "metrics")]
                {
                    line.smsg_ot[RemoveReason::Finished.i()].inc();
                    line.smsg_ob[RemoveReason::Finished.i()].inc_by(bytes);
                }
                r
            },
            _ => self.drain.send(MpscMsg::Event(event)).await,
        }
    }

    async fn flush(
        &mut self,
        _: Bandwidth,
        _: Duration,
    ) -> Result<Bandwidth, ProtocolError<Self::CustomErr>> {
        Ok(0)
    }
}

#[async_trait]
impl<S> RecvProtocol for MpscRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = MpscMsg>,
{
    type CustomErr = S::CustomErr;

    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError<Self::CustomErr>> {
        let event = self.sink.recv().await?;
        #[cfg(feature = "trace_pedantic")]
        trace!(?event, "recv");
        match event {
            MpscMsg::Event(e) => {
                #[cfg(feature = "metrics")]
                {
                    if let ProtocolEvent::Message { data, sid } = &e {
                        let sid = *sid;
                        let bytes = data.len() as u64;
                        let line = self.metrics.init_sid(sid);
                        line.rmsg_it.inc();
                        line.rmsg_ib.inc_by(bytes);
                        line.rmsg_ot[RemoveReason::Finished.i()].inc();
                        line.rmsg_ob[RemoveReason::Finished.i()].inc_by(bytes);
                    }
                }
                Ok(e)
            },
            MpscMsg::InitFrame(_) => Err(ProtocolError::Violated),
        }
    }
}

#[async_trait]
impl<D> ReliableDrain for MpscSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = MpscMsg>,
{
    type CustomErr = D::CustomErr;

    async fn send(&mut self, frame: InitFrame) -> Result<(), ProtocolError<Self::CustomErr>> {
        self.drain.send(MpscMsg::InitFrame(frame)).await
    }
}

#[async_trait]
impl<S> ReliableSink for MpscRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = MpscMsg>,
{
    type CustomErr = S::CustomErr;

    async fn recv(&mut self) -> Result<InitFrame, ProtocolError<Self::CustomErr>> {
        match self.sink.recv().await? {
            MpscMsg::Event(_) => Err(ProtocolError::Violated),
            MpscMsg::InitFrame(f) => Ok(f),
        }
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use crate::metrics::{ProtocolMetricCache, ProtocolMetrics};
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
    ) -> [(MpscSendProtocol<ACDrain>, MpscRecvProtocol<ACSink>); 2] {
        let (s1, r1) = bounded(cap);
        let (s2, r2) = bounded(cap);
        let m = metrics.unwrap_or_else(|| {
            ProtocolMetricCache::new("mpsc", Arc::new(ProtocolMetrics::new().unwrap()))
        });
        [
            (
                MpscSendProtocol::new(ACDrain { sender: s1 }, m.clone()),
                MpscRecvProtocol::new(ACSink { receiver: r2 }, m.clone()),
            ),
            (
                MpscSendProtocol::new(ACDrain { sender: s2 }, m.clone()),
                MpscRecvProtocol::new(ACSink { receiver: r1 }, m),
            ),
        ]
    }

    #[async_trait]
    impl UnreliableDrain for ACDrain {
        type CustomErr = ();
        type DataFormat = MpscMsg;

        async fn send(
            &mut self,
            data: Self::DataFormat,
        ) -> Result<(), ProtocolError<Self::CustomErr>> {
            self.sender
                .send(data)
                .await
                .map_err(|_| ProtocolError::Custom(()))
        }
    }

    #[async_trait]
    impl UnreliableSink for ACSink {
        type CustomErr = ();
        type DataFormat = MpscMsg;

        async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError<Self::CustomErr>> {
            self.receiver
                .recv()
                .await
                .map_err(|_| ProtocolError::Custom(()))
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
