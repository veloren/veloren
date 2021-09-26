use crate::types::Sid;
#[cfg(feature = "metrics")]
use prometheus::{
    core::{AtomicI64, AtomicU64, GenericCounter, GenericGauge},
    IntCounterVec, IntGaugeVec, Opts, Registry,
};
#[cfg(feature = "metrics")]
use std::collections::HashMap;
use std::{error::Error, sync::Arc};

#[allow(dead_code)]
pub enum RemoveReason {
    Finished,
    Dropped,
}

/// Use 1 `ProtocolMetrics` per `Network`.
/// I will contain all protocol related [`prometheus`] information
///
/// [`prometheus`]: prometheus
#[cfg(feature = "metrics")]
pub struct ProtocolMetrics {
    // smsg=send_msg rdata=receive_data
    // i=in o=out
    // t=total b=byte throughput
    //e.g smsg_it = sending messages, in (responsibility of protocol) total

    // based on CHANNEL/STREAM
    /// messages added to be send total, by STREAM,
    smsg_it: IntCounterVec,
    /// messages bytes added  to be send throughput, by STREAM,
    smsg_ib: IntCounterVec,
    /// messages removed from to be send, because they where finished total, by
    /// STREAM AND REASON(finished/canceled),
    smsg_ot: IntCounterVec,
    /// messages bytes removed from to be send throughput, because they where
    /// finished total, by STREAM AND REASON(finished/dropped),
    smsg_ob: IntCounterVec,
    /// data frames send by prio by CHANNEL,
    sdata_frames_t: IntCounterVec,
    /// data frames bytes send by prio by CHANNEL,
    sdata_frames_b: IntCounterVec,

    // based on CHANNEL/STREAM
    /// messages added to be received total, by STREAM,
    rmsg_it: IntCounterVec,
    /// messages bytes added to be received throughput, by STREAM,
    rmsg_ib: IntCounterVec,
    /// messages removed from to be received, because they where finished total,
    /// by STREAM AND REASON(finished/canceled),
    rmsg_ot: IntCounterVec,
    /// messages bytes removed from to be received throughput, because they
    /// where finished total, by STREAM AND REASON(finished/dropped),
    rmsg_ob: IntCounterVec,
    /// data frames send by prio by CHANNEL,
    rdata_frames_t: IntCounterVec,
    /// data frames bytes send by prio by CHANNEL,
    rdata_frames_b: IntCounterVec,
    /// ping per CHANNEL //TODO: implement
    ping: IntGaugeVec,
}

/// Cache for [`ProtocolMetrics`], more optimized and cleared up after channel
/// disconnect.
///
/// [`ProtocolMetrics`]: crate::ProtocolMetrics
#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub struct ProtocolMetricCache {
    cid: String,
    m: Arc<ProtocolMetrics>,
    cache: HashMap<Sid, CacheLine>,
    sdata_frames_t: GenericCounter<AtomicU64>,
    sdata_frames_b: GenericCounter<AtomicU64>,
    rdata_frames_t: GenericCounter<AtomicU64>,
    rdata_frames_b: GenericCounter<AtomicU64>,
    #[allow(dead_code)]
    ping: GenericGauge<AtomicI64>,
}

#[cfg(not(feature = "metrics"))]
#[derive(Debug, Clone)]
pub struct ProtocolMetricCache {}

#[cfg(feature = "metrics")]
impl ProtocolMetrics {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let smsg_it = IntCounterVec::new(
            Opts::new(
                "send_messages_in_total",
                "All Messages that are added to this Protocol to be send at stream level",
            ),
            &["channel", "stream"],
        )?;
        let smsg_ib = IntCounterVec::new(
            Opts::new(
                "send_messages_in_throughput",
                "All Message bytes that are added to this Protocol to be send at stream level",
            ),
            &["channel", "stream"],
        )?;
        let smsg_ot = IntCounterVec::new(
            Opts::new(
                "send_messages_out_total",
                "All Messages that are removed from this Protocol to be send at stream and \
                 reason(finished/canceled) level",
            ),
            &["channel", "stream", "reason"],
        )?;
        let smsg_ob = IntCounterVec::new(
            Opts::new(
                "send_messages_out_throughput",
                "All Message bytes that are removed from this Protocol to be send at stream and \
                 reason(finished/canceled) level",
            ),
            &["channel", "stream", "reason"],
        )?;
        let sdata_frames_t = IntCounterVec::new(
            Opts::new(
                "send_data_frames_total",
                "Number of data frames send per channel",
            ),
            &["channel"],
        )?;
        let sdata_frames_b = IntCounterVec::new(
            Opts::new(
                "send_data_frames_throughput",
                "Number of data frames bytes send per channel",
            ),
            &["channel"],
        )?;

        let rmsg_it = IntCounterVec::new(
            Opts::new(
                "recv_messages_in_total",
                "All Messages that are added to this Protocol to be received at stream level",
            ),
            &["channel", "stream"],
        )?;
        let rmsg_ib = IntCounterVec::new(
            Opts::new(
                "recv_messages_in_throughput",
                "All Message bytes that are added to this Protocol to be received at stream level",
            ),
            &["channel", "stream"],
        )?;
        let rmsg_ot = IntCounterVec::new(
            Opts::new(
                "recv_messages_out_total",
                "All Messages that are removed from this Protocol to be received at stream and \
                 reason(finished/canceled) level",
            ),
            &["channel", "stream", "reason"],
        )?;
        let rmsg_ob = IntCounterVec::new(
            Opts::new(
                "recv_messages_out_throughput",
                "All Message bytes that are removed from this Protocol to be received at stream \
                 and reason(finished/canceled) level",
            ),
            &["channel", "stream", "reason"],
        )?;
        let rdata_frames_t = IntCounterVec::new(
            Opts::new(
                "recv_data_frames_total",
                "Number of data frames received per channel",
            ),
            &["channel"],
        )?;
        let rdata_frames_b = IntCounterVec::new(
            Opts::new(
                "recv_data_frames_throughput",
                "Number of data frames bytes received per channel",
            ),
            &["channel"],
        )?;
        let ping = IntGaugeVec::new(Opts::new("ping", "Ping per channel"), &["channel"])?;

        Ok(Self {
            smsg_it,
            smsg_ib,
            smsg_ot,
            smsg_ob,
            sdata_frames_t,
            sdata_frames_b,
            rmsg_it,
            rmsg_ib,
            rmsg_ot,
            rmsg_ob,
            rdata_frames_t,
            rdata_frames_b,
            ping,
        })
    }

    pub fn register(&self, registry: &Registry) -> Result<(), Box<dyn Error>> {
        registry.register(Box::new(self.smsg_it.clone()))?;
        registry.register(Box::new(self.smsg_ib.clone()))?;
        registry.register(Box::new(self.smsg_ot.clone()))?;
        registry.register(Box::new(self.smsg_ob.clone()))?;
        registry.register(Box::new(self.sdata_frames_t.clone()))?;
        registry.register(Box::new(self.sdata_frames_b.clone()))?;
        registry.register(Box::new(self.rmsg_it.clone()))?;
        registry.register(Box::new(self.rmsg_ib.clone()))?;
        registry.register(Box::new(self.rmsg_ot.clone()))?;
        registry.register(Box::new(self.rmsg_ob.clone()))?;
        registry.register(Box::new(self.rdata_frames_t.clone()))?;
        registry.register(Box::new(self.rdata_frames_b.clone()))?;
        registry.register(Box::new(self.ping.clone()))?;
        Ok(())
    }
}

#[cfg(not(feature = "metrics"))]
pub struct ProtocolMetrics {}

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub(crate) struct CacheLine {
    pub smsg_it: GenericCounter<AtomicU64>,
    pub smsg_ib: GenericCounter<AtomicU64>,
    pub smsg_ot: [GenericCounter<AtomicU64>; 2],
    pub smsg_ob: [GenericCounter<AtomicU64>; 2],
    pub rmsg_it: GenericCounter<AtomicU64>,
    pub rmsg_ib: GenericCounter<AtomicU64>,
    pub rmsg_ot: [GenericCounter<AtomicU64>; 2],
    pub rmsg_ob: [GenericCounter<AtomicU64>; 2],
}

#[cfg(feature = "metrics")]
impl ProtocolMetricCache {
    pub fn new(channel_key: &str, metrics: Arc<ProtocolMetrics>) -> Self {
        let cid = channel_key.to_string();
        let sdata_frames_t = metrics.sdata_frames_t.with_label_values(&[&cid]);
        let sdata_frames_b = metrics.sdata_frames_b.with_label_values(&[&cid]);
        let rdata_frames_t = metrics.rdata_frames_t.with_label_values(&[&cid]);
        let rdata_frames_b = metrics.rdata_frames_b.with_label_values(&[&cid]);
        let ping = metrics.ping.with_label_values(&[&cid]);
        Self {
            cid,
            m: metrics,
            cache: HashMap::new(),
            sdata_frames_t,
            sdata_frames_b,
            rdata_frames_t,
            rdata_frames_b,
            ping,
        }
    }

    pub(crate) fn init_sid(&mut self, sid: Sid) -> &CacheLine {
        let cid = &self.cid;
        let m = &self.m;
        self.cache.entry(sid).or_insert_with_key(|sid| {
            let s = sid.to_string();
            let finished = RemoveReason::Finished.to_str();
            let dropped = RemoveReason::Dropped.to_str();
            CacheLine {
                smsg_it: m.smsg_it.with_label_values(&[cid, &s]),
                smsg_ib: m.smsg_ib.with_label_values(&[cid, &s]),
                smsg_ot: [
                    m.smsg_ot.with_label_values(&[cid, &s, finished]),
                    m.smsg_ot.with_label_values(&[cid, &s, dropped]),
                ],
                smsg_ob: [
                    m.smsg_ob.with_label_values(&[cid, &s, finished]),
                    m.smsg_ob.with_label_values(&[cid, &s, dropped]),
                ],
                rmsg_it: m.rmsg_it.with_label_values(&[cid, &s]),
                rmsg_ib: m.rmsg_ib.with_label_values(&[cid, &s]),
                rmsg_ot: [
                    m.rmsg_ot.with_label_values(&[cid, &s, finished]),
                    m.rmsg_ot.with_label_values(&[cid, &s, dropped]),
                ],
                rmsg_ob: [
                    m.rmsg_ob.with_label_values(&[cid, &s, finished]),
                    m.rmsg_ob.with_label_values(&[cid, &s, dropped]),
                ],
            }
        })
    }

    pub(crate) fn smsg_ib(&mut self, sid: Sid, bytes: u64) {
        let line = self.init_sid(sid);
        line.smsg_it.inc();
        line.smsg_ib.inc_by(bytes);
    }

    pub(crate) fn smsg_ob(&mut self, sid: Sid, reason: RemoveReason, bytes: u64) {
        let line = self.init_sid(sid);
        line.smsg_ot[reason.i()].inc();
        line.smsg_ob[reason.i()].inc_by(bytes);
    }

    pub(crate) fn sdata_frames_b(&mut self, cnt: u64, bytes: u64) {
        self.sdata_frames_t.inc_by(cnt);
        self.sdata_frames_b.inc_by(bytes);
    }

    pub(crate) fn rmsg_ib(&mut self, sid: Sid, bytes: u64) {
        let line = self.init_sid(sid);
        line.rmsg_it.inc();
        line.rmsg_ib.inc_by(bytes);
    }

    pub(crate) fn rmsg_ob(&mut self, sid: Sid, reason: RemoveReason, bytes: u64) {
        let line = self.init_sid(sid);
        line.rmsg_ot[reason.i()].inc();
        line.rmsg_ob[reason.i()].inc_by(bytes);
    }

    pub(crate) fn rdata_frames_b(&mut self, bytes: u64) {
        self.rdata_frames_t.inc();
        self.rdata_frames_b.inc_by(bytes);
    }

    #[cfg(test)]
    pub(crate) fn assert_msg(&mut self, sid: Sid, cnt: u64, reason: RemoveReason) {
        let line = self.init_sid(sid);
        assert_eq!(line.smsg_it.get(), cnt);
        assert_eq!(line.smsg_ot[reason.i()].get(), cnt);
        assert_eq!(line.rmsg_it.get(), cnt);
        assert_eq!(line.rmsg_ot[reason.i()].get(), cnt);
    }

    #[cfg(test)]
    pub(crate) fn assert_msg_bytes(&mut self, sid: Sid, bytes: u64, reason: RemoveReason) {
        let line = self.init_sid(sid);
        assert_eq!(line.smsg_ib.get(), bytes);
        assert_eq!(line.smsg_ob[reason.i()].get(), bytes);
        assert_eq!(line.rmsg_ib.get(), bytes);
        assert_eq!(line.rmsg_ob[reason.i()].get(), bytes);
    }

    #[cfg(test)]
    pub(crate) fn assert_data_frames(&mut self, cnt: u64) {
        assert_eq!(self.sdata_frames_t.get(), cnt);
        assert_eq!(self.rdata_frames_t.get(), cnt);
    }

    #[cfg(test)]
    pub(crate) fn assert_data_frames_bytes(&mut self, bytes: u64) {
        assert_eq!(self.sdata_frames_b.get(), bytes);
        assert_eq!(self.rdata_frames_b.get(), bytes);
    }
}

#[cfg(feature = "metrics")]
impl Drop for ProtocolMetricCache {
    fn drop(&mut self) {
        let cid = &self.cid;
        let m = &self.m;
        let finished = RemoveReason::Finished.to_str();
        let dropped = RemoveReason::Dropped.to_str();
        for (sid, _) in self.cache.drain() {
            let s = sid.to_string();
            let _ = m.smsg_it.remove_label_values(&[cid, &s]);
            let _ = m.smsg_ib.remove_label_values(&[cid, &s]);
            let _ = m.smsg_ot.remove_label_values(&[cid, &s, finished]);
            let _ = m.smsg_ot.remove_label_values(&[cid, &s, dropped]);
            let _ = m.smsg_ob.remove_label_values(&[cid, &s, finished]);
            let _ = m.smsg_ob.remove_label_values(&[cid, &s, dropped]);
            let _ = m.rmsg_it.remove_label_values(&[cid, &s]);
            let _ = m.rmsg_ib.remove_label_values(&[cid, &s]);
            let _ = m.rmsg_ot.remove_label_values(&[cid, &s, finished]);
            let _ = m.rmsg_ot.remove_label_values(&[cid, &s, dropped]);
            let _ = m.rmsg_ob.remove_label_values(&[cid, &s, finished]);
            let _ = m.rmsg_ob.remove_label_values(&[cid, &s, dropped]);
        }
        let _ = m.ping.remove_label_values(&[cid]);
        let _ = m.sdata_frames_t.remove_label_values(&[cid]);
        let _ = m.sdata_frames_b.remove_label_values(&[cid]);
        let _ = m.rdata_frames_t.remove_label_values(&[cid]);
        let _ = m.rdata_frames_b.remove_label_values(&[cid]);
    }
}

#[cfg(feature = "metrics")]
impl std::fmt::Debug for ProtocolMetrics {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProtocolMetrics()")
    }
}

#[cfg(not(feature = "metrics"))]
impl ProtocolMetricCache {
    pub fn new(_channel_key: &str, _metrics: Arc<ProtocolMetrics>) -> Self { Self {} }

    pub(crate) fn smsg_ib(&mut self, _sid: Sid, _b: u64) {}

    pub(crate) fn smsg_ob(&mut self, _sid: Sid, _reason: RemoveReason, _b: u64) {}

    pub(crate) fn sdata_frames_b(&mut self, _cnt: u64, _b: u64) {}

    pub(crate) fn rmsg_ib(&mut self, _sid: Sid, _b: u64) {}

    pub(crate) fn rmsg_ob(&mut self, _sid: Sid, _reason: RemoveReason, _b: u64) {}

    pub(crate) fn rdata_frames_b(&mut self, _b: u64) {}
}

#[cfg(not(feature = "metrics"))]
impl ProtocolMetrics {
    pub fn new() -> Result<Self, Box<dyn Error>> { Ok(Self {}) }
}

impl RemoveReason {
    #[cfg(feature = "metrics")]
    fn to_str(&self) -> &str {
        match self {
            RemoveReason::Finished => "Finished",
            RemoveReason::Dropped => "Dropped",
        }
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn i(&self) -> usize {
        match self {
            RemoveReason::Finished => 0,
            RemoveReason::Dropped => 1,
        }
    }
}
