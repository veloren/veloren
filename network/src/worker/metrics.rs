use prometheus::{IntGauge, IntGaugeVec, Opts, Registry};
use std::{
    error::Error,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

// 1 NetworkMetrics per Network
pub struct NetworkMetrics {
    pub participants_connected: IntGauge,
    pub channels_connected: IntGauge,
    pub streams_open: IntGauge,
    pub worker_count: IntGauge,
    pub network_info: IntGauge,
    // Frames, seperated by CHANNEL (add PART and PROTOCOL) AND FRAME TYPE,
    pub frames_count: IntGaugeVec,
    // send Messages, seperated by STREAM (add PART and PROTOCOL, CHANNEL),
    pub message_count: IntGaugeVec,
    // send Messages bytes, seperated by STREAM (add PART and PROTOCOL, CHANNEL),
    pub bytes_send: IntGaugeVec,
    // queued Messages, seperated by STREAM (add PART and PROTOCOL, CHANNEL),
    pub queue_count: IntGaugeVec,
    // worker seperated by CHANNEL (add PART and PROTOCOL),
    pub worker_work_time: IntGaugeVec,
    // worker seperated by CHANNEL (add PART and PROTOCOL),
    pub worker_idle_time: IntGaugeVec,
    // ping calculated based on last msg
    pub participants_ping: IntGaugeVec,
    tick: Arc<AtomicU64>,
}

impl NetworkMetrics {
    pub fn new(registry: &Registry, tick: Arc<AtomicU64>) -> Result<Self, Box<dyn Error>> {
        let participants_connected = IntGauge::with_opts(Opts::new(
            "participants_connected",
            "shows the number of participants connected to the network",
        ))?;
        let channels_connected = IntGauge::with_opts(Opts::new(
            "channels_connected",
            "number of all channels currently connected on the network",
        ))?;
        let streams_open = IntGauge::with_opts(Opts::new(
            "streams_open",
            "number of all streams currently open on the network",
        ))?;
        let worker_count = IntGauge::with_opts(Opts::new(
            "worker_count",
            "number of workers currently running",
        ))?;
        let opts = Opts::new("network_info", "Static Network information").const_label(
            "version",
            &format!(
                "{}.{}.{}",
                &crate::internal::VELOREN_NETWORK_VERSION[0],
                &crate::internal::VELOREN_NETWORK_VERSION[1],
                &crate::internal::VELOREN_NETWORK_VERSION[2]
            ),
        );
        let network_info = IntGauge::with_opts(opts)?;

        let frames_count = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "frames_count",
                "time in ns requiered for a tick of the server",
            ),
            &["channel"],
        )?);
        let message_count = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "message_count",
                "time in ns requiered for a tick of the server",
            ),
            &["channel"],
        )?);
        let bytes_send = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "bytes_send",
                "time in ns requiered for a tick of the server",
            ),
            &["channel"],
        )?);
        let queue_count = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "queue_count",
                "time in ns requiered for a tick of the server",
            ),
            &["channel"],
        )?);
        let worker_work_time = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "worker_work_time",
                "time in ns requiered for a tick of the server",
            ),
            &["channel"],
        )?);
        let worker_idle_time = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "worker_idle_time",
                "time in ns requiered for a tick of the server",
            ),
            &["channel"],
        )?);
        let participants_ping = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "participants_ping",
                "time in ns requiered for a tick of the server",
            ),
            &["channel"],
        )?);

        registry.register(Box::new(participants_connected.clone()))?;
        registry.register(Box::new(channels_connected.clone()))?;
        registry.register(Box::new(streams_open.clone()))?;
        registry.register(Box::new(worker_count.clone()))?;
        registry.register(Box::new(network_info.clone()))?;
        registry.register(Box::new(frames_count.clone()))?;
        registry.register(Box::new(message_count.clone()))?;
        registry.register(Box::new(bytes_send.clone()))?;
        registry.register(Box::new(queue_count.clone()))?;
        registry.register(Box::new(worker_work_time.clone()))?;
        registry.register(Box::new(worker_idle_time.clone()))?;
        registry.register(Box::new(participants_ping.clone()))?;

        Ok(Self {
            participants_connected,
            channels_connected,
            streams_open,
            worker_count,
            network_info,
            frames_count,
            message_count,
            bytes_send,
            queue_count,
            worker_work_time,
            worker_idle_time,
            participants_ping,
            tick,
        })
    }

    pub fn is_100th_tick(&self) -> bool { self.tick.load(Ordering::Relaxed).rem_euclid(100) == 0 }
}
