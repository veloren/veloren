use crate::types::Sid;
#[cfg(feature = "metrics")]
use prometheus::{IntCounterVec, IntGaugeVec, Opts, Registry};
#[cfg(feature = "metrics")]
use std::{error::Error, sync::Arc};

#[allow(dead_code)]
pub enum RemoveReason {
    Finished,
    Dropped,
}

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

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub struct ProtocolMetricCache {
    cid: String,
    m: Arc<ProtocolMetrics>,
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

#[cfg(feature = "metrics")]
impl ProtocolMetricCache {
    pub fn new(channel_key: &str, metrics: Arc<ProtocolMetrics>) -> Self {
        Self {
            cid: channel_key.to_string(),
            m: metrics,
        }
    }

    pub(crate) fn smsg_it(&self, sid: Sid) {
        self.m
            .smsg_it
            .with_label_values(&[&self.cid, &sid.to_string()])
            .inc();
    }

    pub(crate) fn smsg_ib(&self, sid: Sid, bytes: u64) {
        self.m
            .smsg_ib
            .with_label_values(&[&self.cid, &sid.to_string()])
            .inc_by(bytes);
    }

    pub(crate) fn smsg_ot(&self, sid: Sid, reason: RemoveReason) {
        self.m
            .smsg_ot
            .with_label_values(&[&self.cid, &sid.to_string(), reason.to_str()])
            .inc();
    }

    pub(crate) fn smsg_ob(&self, sid: Sid, reason: RemoveReason, bytes: u64) {
        self.m
            .smsg_ob
            .with_label_values(&[&self.cid, &sid.to_string(), reason.to_str()])
            .inc_by(bytes);
    }

    pub(crate) fn sdata_frames_t(&self) {
        self.m.sdata_frames_t.with_label_values(&[&self.cid]).inc();
    }

    pub(crate) fn sdata_frames_b(&self, bytes: u64) {
        self.m
            .sdata_frames_b
            .with_label_values(&[&self.cid])
            .inc_by(bytes);
    }

    pub(crate) fn rmsg_it(&self, sid: Sid) {
        self.m
            .rmsg_it
            .with_label_values(&[&self.cid, &sid.to_string()])
            .inc();
    }

    pub(crate) fn rmsg_ib(&self, sid: Sid, bytes: u64) {
        self.m
            .rmsg_ib
            .with_label_values(&[&self.cid, &sid.to_string()])
            .inc_by(bytes);
    }

    pub(crate) fn rmsg_ot(&self, sid: Sid, reason: RemoveReason) {
        self.m
            .rmsg_ot
            .with_label_values(&[&self.cid, &sid.to_string(), reason.to_str()])
            .inc();
    }

    pub(crate) fn rmsg_ob(&self, sid: Sid, reason: RemoveReason, bytes: u64) {
        self.m
            .rmsg_ob
            .with_label_values(&[&self.cid, &sid.to_string(), reason.to_str()])
            .inc_by(bytes);
    }

    pub(crate) fn rdata_frames_t(&self) {
        self.m.rdata_frames_t.with_label_values(&[&self.cid]).inc();
    }

    pub(crate) fn rdata_frames_b(&self, bytes: u64) {
        self.m
            .rdata_frames_b
            .with_label_values(&[&self.cid])
            .inc_by(bytes);
    }

    #[cfg(test)]
    pub(crate) fn assert_msg(&self, sid: Sid, cnt: u64, reason: RemoveReason) {
        assert_eq!(
            self.m
                .smsg_it
                .with_label_values(&[&self.cid, &sid.to_string()])
                .get(),
            cnt
        );
        assert_eq!(
            self.m
                .smsg_ot
                .with_label_values(&[&self.cid, &sid.to_string(), reason.to_str()])
                .get(),
            cnt
        );
        assert_eq!(
            self.m
                .rmsg_it
                .with_label_values(&[&self.cid, &sid.to_string()])
                .get(),
            cnt
        );
        assert_eq!(
            self.m
                .rmsg_ot
                .with_label_values(&[&self.cid, &sid.to_string(), reason.to_str()])
                .get(),
            cnt
        );
    }

    #[cfg(test)]
    pub(crate) fn assert_msg_bytes(&self, sid: Sid, bytes: u64, reason: RemoveReason) {
        assert_eq!(
            self.m
                .smsg_ib
                .with_label_values(&[&self.cid, &sid.to_string()])
                .get(),
            bytes
        );
        assert_eq!(
            self.m
                .smsg_ob
                .with_label_values(&[&self.cid, &sid.to_string(), reason.to_str()])
                .get(),
            bytes
        );
        assert_eq!(
            self.m
                .rmsg_ib
                .with_label_values(&[&self.cid, &sid.to_string()])
                .get(),
            bytes
        );
        assert_eq!(
            self.m
                .rmsg_ob
                .with_label_values(&[&self.cid, &sid.to_string(), reason.to_str()])
                .get(),
            bytes
        );
    }

    #[cfg(test)]
    pub(crate) fn assert_data_frames(&self, cnt: u64) {
        assert_eq!(
            self.m.sdata_frames_t.with_label_values(&[&self.cid]).get(),
            cnt
        );
        assert_eq!(
            self.m.rdata_frames_t.with_label_values(&[&self.cid]).get(),
            cnt
        );
    }

    #[cfg(test)]
    pub(crate) fn assert_data_frames_bytes(&self, bytes: u64) {
        assert_eq!(
            self.m.sdata_frames_b.with_label_values(&[&self.cid]).get(),
            bytes
        );
        assert_eq!(
            self.m.rdata_frames_b.with_label_values(&[&self.cid]).get(),
            bytes
        );
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
    pub(crate) fn smsg_it(&self, _sid: Sid) {}

    pub(crate) fn smsg_ib(&self, _sid: Sid, _b: u64) {}

    pub(crate) fn smsg_ot(&self, _sid: Sid, _reason: RemoveReason) {}

    pub(crate) fn smsg_ob(&self, _sid: Sid, _reason: RemoveReason, _b: u64) {}

    pub(crate) fn sdata_frames_t(&self) {}

    pub(crate) fn sdata_frames_b(&self, _b: u64) {}

    pub(crate) fn rmsg_it(&self, _sid: Sid) {}

    pub(crate) fn rmsg_ib(&self, _sid: Sid, _b: u64) {}

    pub(crate) fn rmsg_ot(&self, _sid: Sid, _reason: RemoveReason) {}

    pub(crate) fn rmsg_ob(&self, _sid: Sid, _reason: RemoveReason, _b: u64) {}

    pub(crate) fn rdata_frames_t(&self) {}

    pub(crate) fn rdata_frames_b(&self, _b: u64) {}
}

impl RemoveReason {
    #[cfg(feature = "metrics")]
    fn to_str(&self) -> &str {
        match self {
            RemoveReason::Dropped => "Dropped",
            RemoveReason::Finished => "Finished",
        }
    }
}
