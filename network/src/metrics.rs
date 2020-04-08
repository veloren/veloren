use prometheus::{IntGauge, IntGaugeVec, Opts, Registry};
use std::{
    error::Error,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

//TODO: switch over to Counter for frames_count, message_count, bytes_send,
// frames_message_count 1 NetworkMetrics per Network
#[allow(dead_code)]
pub struct NetworkMetrics {
    pub participants_connected: IntGauge,
    // opened Channels, seperated by PARTICIPANT
    pub channels_connected: IntGauge,
    // opened streams, seperated by PARTICIPANT
    pub streams_open: IntGauge,
    pub network_info: IntGauge,
    // Frames, seperated by CHANNEL (and PARTICIPANT) AND FRAME TYPE,
    pub frames_count: IntGaugeVec,
    // send Messages, seperated by STREAM (and PARTICIPANT, CHANNEL),
    pub message_count: IntGaugeVec,
    // send Messages bytes, seperated by STREAM (and PARTICIPANT, CHANNEL),
    pub bytes_send: IntGaugeVec,
    // Frames, seperated by MESSAGE (and PARTICIPANT, CHANNEL, STREAM),
    pub frames_message_count: IntGaugeVec,
    // TODO: queued Messages, seperated by STREAM (add PART, CHANNEL),
    // queued Messages, seperated by PARTICIPANT
    pub queued_count: IntGaugeVec,
    // TODO: queued Messages bytes, seperated by STREAM (add PART, CHANNEL),
    // queued Messages bytes, seperated by PARTICIPANT
    pub queued_bytes: IntGaugeVec,
    // ping calculated based on last msg seperated by PARTICIPANT
    pub participants_ping: IntGaugeVec,
    tick: Arc<AtomicU64>,
}

impl NetworkMetrics {
    #[allow(dead_code)]
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
        let opts = Opts::new("network_info", "Static Network information").const_label(
            "version",
            &format!(
                "{}.{}.{}",
                &crate::types::VELOREN_NETWORK_VERSION[0],
                &crate::types::VELOREN_NETWORK_VERSION[1],
                &crate::types::VELOREN_NETWORK_VERSION[2]
            ),
        );
        let network_info = IntGauge::with_opts(opts)?;

        let frames_count = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "frames_count",
                "number of all frames send by streams on the network",
            ),
            &["channel"],
        )?);
        let message_count = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "message_count",
                "number of messages send by streams on the network",
            ),
            &["channel"],
        )?);
        let bytes_send = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new("bytes_send", "bytes send by streams on the network"),
            &["channel"],
        )?);
        let frames_message_count = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "frames_message_count",
                "bytes sends per message on the network",
            ),
            &["channel"],
        )?);
        let queued_count = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "queued_count",
                "queued number of messages by participant on the network",
            ),
            &["channel"],
        )?);
        let queued_bytes = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "queued_bytes",
                "queued bytes of messages by participant on the network",
            ),
            &["channel"],
        )?);
        let participants_ping = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new(
                "participants_ping",
                "ping time to participants on the network",
            ),
            &["channel"],
        )?);

        registry.register(Box::new(participants_connected.clone()))?;
        registry.register(Box::new(channels_connected.clone()))?;
        registry.register(Box::new(streams_open.clone()))?;
        registry.register(Box::new(network_info.clone()))?;
        registry.register(Box::new(frames_count.clone()))?;
        registry.register(Box::new(message_count.clone()))?;
        registry.register(Box::new(bytes_send.clone()))?;
        registry.register(Box::new(frames_message_count.clone()))?;
        registry.register(Box::new(queued_count.clone()))?;
        registry.register(Box::new(queued_bytes.clone()))?;
        registry.register(Box::new(participants_ping.clone()))?;

        Ok(Self {
            participants_connected,
            channels_connected,
            streams_open,
            network_info,
            frames_count,
            message_count,
            bytes_send,
            frames_message_count,
            queued_count,
            queued_bytes,
            participants_ping,
            tick,
        })
    }

    pub fn _is_100th_tick(&self) -> bool { self.tick.load(Ordering::Relaxed).rem_euclid(100) == 0 }
}
