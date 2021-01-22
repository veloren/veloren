use network_protocol::Pid;
use prometheus::{IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry};
use std::error::Error;

/// 1:1 relation between NetworkMetrics and Network
/// use 2NF here and avoid redundant data like CHANNEL AND PARTICIPANT encoding.
/// as this will cause a matrix that is full of 0 but needs alot of bandwith and
/// storage
#[allow(dead_code)]
pub struct NetworkMetrics {
    pub listen_requests_total: IntCounterVec,
    pub connect_requests_total: IntCounterVec,
    pub participants_connected_total: IntCounter,
    pub participants_disconnected_total: IntCounter,
    // channel id's, seperated by PARTICIPANT, max 5
    pub participants_channel_ids: IntGaugeVec,
    // opened Channels, seperated by PARTICIPANT
    pub channels_connected_total: IntCounterVec,
    pub channels_disconnected_total: IntCounterVec,
    // opened streams, seperated by PARTICIPANT
    pub streams_opened_total: IntCounterVec,
    pub streams_closed_total: IntCounterVec,
    pub network_info: IntGauge,
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
        let participants_channel_ids = IntGaugeVec::new(
            Opts::new(
                "participants_channel_ids",
                "Channel numbers belonging to a Participant in the network",
            ),
            &["participant", "no"],
        )?;
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
                    &network_protocol::VELOREN_NETWORK_VERSION[0],
                    &network_protocol::VELOREN_NETWORK_VERSION[1],
                    &network_protocol::VELOREN_NETWORK_VERSION[2]
                ),
            )
            .const_label("local_pid", &format!("{}", &local_pid));
        let network_info = IntGauge::with_opts(opts)?;

        Ok(Self {
            listen_requests_total,
            connect_requests_total,
            participants_connected_total,
            participants_disconnected_total,
            participants_channel_ids,
            channels_connected_total,
            channels_disconnected_total,
            streams_opened_total,
            streams_closed_total,
            network_info,
        })
    }

    pub fn register(&self, registry: &Registry) -> Result<(), Box<dyn Error>> {
        registry.register(Box::new(self.listen_requests_total.clone()))?;
        registry.register(Box::new(self.connect_requests_total.clone()))?;
        registry.register(Box::new(self.participants_connected_total.clone()))?;
        registry.register(Box::new(self.participants_disconnected_total.clone()))?;
        registry.register(Box::new(self.participants_channel_ids.clone()))?;
        registry.register(Box::new(self.channels_connected_total.clone()))?;
        registry.register(Box::new(self.channels_disconnected_total.clone()))?;
        registry.register(Box::new(self.streams_opened_total.clone()))?;
        registry.register(Box::new(self.streams_closed_total.clone()))?;
        registry.register(Box::new(self.network_info.clone()))?;
        Ok(())
    }
}

impl std::fmt::Debug for NetworkMetrics {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NetworkMetrics()")
    }
}
