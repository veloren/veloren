use crate::types::{Cid, Frame, Pid};
use prometheus::{
    core::{AtomicI64, GenericCounter},
    IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use std::error::Error;
use tracing::*;

//TODO: switch over to Counter for frames_count, message_count, bytes_send,
// frames_message_count 1 NetworkMetrics per Network
#[allow(dead_code)]
pub struct NetworkMetrics {
    pub listen_requests_total: IntCounterVec,
    pub connect_requests_total: IntCounterVec,
    pub participants_connected_total: IntCounter,
    pub participants_disconnected_total: IntCounter,
    // opened Channels, seperated by PARTICIPANT
    pub channels_connected_total: IntCounterVec,
    pub channels_disconnected_total: IntCounterVec,
    // opened streams, seperated by PARTICIPANT
    pub streams_opened_total: IntCounterVec,
    pub streams_closed_total: IntCounterVec,
    pub network_info: IntGauge,
    // Frames counted a channel level, seperated by CHANNEL (and PARTICIPANT) AND FRAME TYPE,
    pub frames_out_total: IntCounterVec,
    pub frames_in_total: IntCounterVec,
    // Frames counted at protocol level, seperated by CHANNEL (and PARTICIPANT) AND FRAME TYPE,
    pub frames_wire_out_total: IntCounterVec,
    pub frames_wire_in_total: IntCounterVec,
    // throughput at protocol level, seperated by CHANNEL (and PARTICIPANT),
    pub wire_out_throughput: IntCounterVec,
    pub wire_in_throughput: IntCounterVec,
    // send(prio) Messages count, seperated by STREAM AND PARTICIPANT,
    pub message_out_total: IntCounterVec,
    // send(prio) Messages throughput, seperated by STREAM AND PARTICIPANT,
    pub message_out_throughput: IntCounterVec,
    // TODO: queued Messages, seperated by STREAM (add PART, CHANNEL),
    // queued Messages, seperated by PARTICIPANT
    pub queued_count: IntGaugeVec,
    // TODO: queued Messages bytes, seperated by STREAM (add PART, CHANNEL),
    // queued Messages bytes, seperated by PARTICIPANT
    pub queued_bytes: IntGaugeVec,
    // ping calculated based on last msg seperated by PARTICIPANT
    pub participants_ping: IntGaugeVec,
}

impl NetworkMetrics {
    #[allow(dead_code)]
    pub fn new(local_pid: &Pid) -> Result<Self, Box<dyn Error>> {
        let listen_requests_total = IntCounterVec::new(
            Opts::new(
                "listen_requests_total",
                "Shows the number of listen requests to the scheduler",
            ),
            &["protocol"],
        )?;
        let connect_requests_total = IntCounterVec::new(
            Opts::new(
                "connect_requests_total",
                "Shows the number of connect requests to the scheduler",
            ),
            &["protocol"],
        )?;
        let participants_connected_total = IntCounter::with_opts(Opts::new(
            "participants_connected_total",
            "Shows the number of participants connected to the network",
        ))?;
        let participants_disconnected_total = IntCounter::with_opts(Opts::new(
            "participants_disconnected_total",
            "Shows the number of participants disconnected to the network",
        ))?;
        let channels_connected_total = IntCounterVec::new(
            Opts::new(
                "channels_connected_total",
                "Number of all channels currently connected on the network",
            ),
            &["participant"],
        )?;
        let channels_disconnected_total = IntCounterVec::new(
            Opts::new(
                "channels_disconnected_total",
                "Number of all channels currently disconnected on the network",
            ),
            &["participant"],
        )?;
        let streams_opened_total = IntCounterVec::new(
            Opts::new(
                "streams_opened_total",
                "Number of all streams currently open on the network",
            ),
            &["participant"],
        )?;
        let streams_closed_total = IntCounterVec::new(
            Opts::new(
                "streams_closed_total",
                "Number of all streams currently open on the network",
            ),
            &["participant"],
        )?;
        let opts = Opts::new("network_info", "Static Network information")
            .const_label(
                "version",
                &format!(
                    "{}.{}.{}",
                    &crate::types::VELOREN_NETWORK_VERSION[0],
                    &crate::types::VELOREN_NETWORK_VERSION[1],
                    &crate::types::VELOREN_NETWORK_VERSION[2]
                ),
            )
            .const_label("local_pid", &format!("{}", &local_pid));
        let network_info = IntGauge::with_opts(opts)?;
        let frames_out_total = IntCounterVec::new(
            Opts::new(
                "frames_out_total",
                "Number of all frames send per channel, at the channel level",
            ),
            &["participant", "channel", "frametype"],
        )?;
        let frames_in_total = IntCounterVec::new(
            Opts::new(
                "frames_in_total",
                "Number of all frames received per channel, at the channel level",
            ),
            &["participant", "channel", "frametype"],
        )?;
        let frames_wire_out_total = IntCounterVec::new(
            Opts::new(
                "frames_wire_out_total",
                "Number of all frames send per channel, at the protocol level",
            ),
            &["channel", "frametype"],
        )?;
        let frames_wire_in_total = IntCounterVec::new(
            Opts::new(
                "frames_wire_in_total",
                "Number of all frames received per channel, at the protocol level",
            ),
            &["channel", "frametype"],
        )?;
        let wire_out_throughput = IntCounterVec::new(
            Opts::new(
                "wire_out_throughput",
                "Throupgput of all data frames send per channel, at the protocol level",
            ),
            &["channel"],
        )?;
        let wire_in_throughput = IntCounterVec::new(
            Opts::new(
                "wire_in_throughput",
                "Throupgput of all data frames send per channel, at the protocol level",
            ),
            &["channel"],
        )?;
        //TODO IN
        let message_out_total = IntCounterVec::new(
            Opts::new(
                "message_out_total",
                "Number of messages send by streams on the network",
            ),
            &["participant", "stream"],
        )?;
        //TODO IN
        let message_out_throughput = IntCounterVec::new(
            Opts::new(
                "message_out_throughput",
                "Throughput of messages send by streams on the network",
            ),
            &["participant", "stream"],
        )?;
        let queued_count = IntGaugeVec::new(
            Opts::new(
                "queued_count",
                "Queued number of messages by participant on the network",
            ),
            &["channel"],
        )?;
        let queued_bytes = IntGaugeVec::new(
            Opts::new(
                "queued_bytes",
                "Queued bytes of messages by participant on the network",
            ),
            &["channel"],
        )?;
        let participants_ping = IntGaugeVec::new(
            Opts::new(
                "participants_ping",
                "Ping time to participants on the network",
            ),
            &["channel"],
        )?;

        Ok(Self {
            listen_requests_total,
            connect_requests_total,
            participants_connected_total,
            participants_disconnected_total,
            channels_connected_total,
            channels_disconnected_total,
            streams_opened_total,
            streams_closed_total,
            network_info,
            frames_out_total,
            frames_in_total,
            frames_wire_out_total,
            frames_wire_in_total,
            wire_out_throughput,
            wire_in_throughput,
            message_out_total,
            message_out_throughput,
            queued_count,
            queued_bytes,
            participants_ping,
        })
    }

    pub fn register(&self, registry: &Registry) -> Result<(), Box<dyn Error>> {
        registry.register(Box::new(self.listen_requests_total.clone()))?;
        registry.register(Box::new(self.connect_requests_total.clone()))?;
        registry.register(Box::new(self.participants_connected_total.clone()))?;
        registry.register(Box::new(self.participants_disconnected_total.clone()))?;
        registry.register(Box::new(self.channels_connected_total.clone()))?;
        registry.register(Box::new(self.channels_disconnected_total.clone()))?;
        registry.register(Box::new(self.streams_opened_total.clone()))?;
        registry.register(Box::new(self.streams_closed_total.clone()))?;
        registry.register(Box::new(self.network_info.clone()))?;
        registry.register(Box::new(self.frames_out_total.clone()))?;
        registry.register(Box::new(self.frames_in_total.clone()))?;
        registry.register(Box::new(self.frames_wire_out_total.clone()))?;
        registry.register(Box::new(self.frames_wire_in_total.clone()))?;
        registry.register(Box::new(self.wire_out_throughput.clone()))?;
        registry.register(Box::new(self.wire_in_throughput.clone()))?;
        registry.register(Box::new(self.message_out_total.clone()))?;
        registry.register(Box::new(self.message_out_throughput.clone()))?;
        registry.register(Box::new(self.queued_count.clone()))?;
        registry.register(Box::new(self.queued_bytes.clone()))?;
        registry.register(Box::new(self.participants_ping.clone()))?;
        Ok(())
    }

    //pub fn _is_100th_tick(&self) -> bool {
    // self.tick.load(Ordering::Relaxed).rem_euclid(100) == 0 }
}

impl std::fmt::Debug for NetworkMetrics {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NetworkMetrics()")
    }
}

/*
pub(crate) struct PidCidFrameCache<T: MetricVecBuilder> {
    metric: MetricVec<T>,
    pid: String,
    cache: Vec<[T::M; 8]>,
}
*/

pub(crate) struct PidCidFrameCache {
    metric: IntCounterVec,
    pid: String,
    cache: Vec<[GenericCounter<AtomicI64>; Frame::FRAMES_LEN as usize]>,
}

impl PidCidFrameCache {
    const CACHE_SIZE: usize = 512;

    pub fn new(metric: IntCounterVec, pid: Pid) -> Self {
        Self {
            metric,
            pid: pid.to_string(),
            cache: vec![],
        }
    }

    fn populate(&mut self, cid: Cid) {
        let start_cid = self.cache.len();
        if cid >= start_cid as u64 && cid > (Self::CACHE_SIZE as Cid) {
            warn!(
                ?cid,
                "cid, getting quite high, is this a attack on the cache?"
            );
        }
        for i in start_cid..=cid as usize {
            let cid = (i as Cid).to_string();
            let entry = [
                self.metric
                    .with_label_values(&[&self.pid, &cid, Frame::int_to_string(0)]),
                self.metric
                    .with_label_values(&[&self.pid, &cid, Frame::int_to_string(1)]),
                self.metric
                    .with_label_values(&[&self.pid, &cid, Frame::int_to_string(2)]),
                self.metric
                    .with_label_values(&[&self.pid, &cid, Frame::int_to_string(3)]),
                self.metric
                    .with_label_values(&[&self.pid, &cid, Frame::int_to_string(4)]),
                self.metric
                    .with_label_values(&[&self.pid, &cid, Frame::int_to_string(5)]),
                self.metric
                    .with_label_values(&[&self.pid, &cid, Frame::int_to_string(6)]),
                self.metric
                    .with_label_values(&[&self.pid, &cid, Frame::int_to_string(7)]),
            ];
            self.cache.push(entry);
        }
    }

    pub fn with_label_values(&mut self, cid: Cid, frame: &Frame) -> &GenericCounter<AtomicI64> {
        self.populate(cid);
        &self.cache[cid as usize][frame.get_int() as usize]
    }
}

pub(crate) struct CidFrameCache {
    cache: [GenericCounter<AtomicI64>; Frame::FRAMES_LEN as usize],
}

impl CidFrameCache {
    pub fn new(metric: IntCounterVec, cid: Cid) -> Self {
        let cid = cid.to_string();
        let cache = [
            metric.with_label_values(&[&cid, Frame::int_to_string(0)]),
            metric.with_label_values(&[&cid, Frame::int_to_string(1)]),
            metric.with_label_values(&[&cid, Frame::int_to_string(2)]),
            metric.with_label_values(&[&cid, Frame::int_to_string(3)]),
            metric.with_label_values(&[&cid, Frame::int_to_string(4)]),
            metric.with_label_values(&[&cid, Frame::int_to_string(5)]),
            metric.with_label_values(&[&cid, Frame::int_to_string(6)]),
            metric.with_label_values(&[&cid, Frame::int_to_string(7)]),
        ];
        Self { cache }
    }

    pub fn with_label_values(&mut self, frame: &Frame) -> &GenericCounter<AtomicI64> {
        &self.cache[frame.get_int() as usize]
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        metrics::*,
        types::{Frame, Pid},
    };

    #[test]
    fn register_metrics() {
        let registry = Registry::new();
        let metrics = NetworkMetrics::new(&Pid::fake(1)).unwrap();
        metrics.register(&registry).unwrap();
    }

    #[test]
    fn pid_cid_frame_cache() {
        let pid = Pid::fake(1);
        let frame1 = Frame::Raw(b"Foo".to_vec());
        let frame2 = Frame::Raw(b"Bar".to_vec());
        let metrics = NetworkMetrics::new(&pid).unwrap();
        let mut cache = PidCidFrameCache::new(metrics.frames_in_total, pid);
        let v1 = cache.with_label_values(1, &frame1);
        v1.inc();
        assert_eq!(v1.get(), 1);
        let v2 = cache.with_label_values(1, &frame1);
        v2.inc();
        assert_eq!(v2.get(), 2);
        let v3 = cache.with_label_values(1, &frame2);
        v3.inc();
        assert_eq!(v3.get(), 3);
        let v4 = cache.with_label_values(3, &frame1);
        v4.inc();
        assert_eq!(v4.get(), 1);
        let v5 = cache.with_label_values(3, &Frame::Shutdown);
        v5.inc();
        assert_eq!(v5.get(), 1);
    }

    #[test]
    fn cid_frame_cache() {
        let pid = Pid::fake(1);
        let frame1 = Frame::Raw(b"Foo".to_vec());
        let frame2 = Frame::Raw(b"Bar".to_vec());
        let metrics = NetworkMetrics::new(&pid).unwrap();
        let mut cache = CidFrameCache::new(metrics.frames_wire_out_total, 1);
        let v1 = cache.with_label_values(&frame1);
        v1.inc();
        assert_eq!(v1.get(), 1);
        let v2 = cache.with_label_values(&frame1);
        v2.inc();
        assert_eq!(v2.get(), 2);
        let v3 = cache.with_label_values(&frame2);
        v3.inc();
        assert_eq!(v3.get(), 3);
        let v4 = cache.with_label_values(&Frame::Shutdown);
        v4.inc();
        assert_eq!(v4.get(), 1);
    }
}
