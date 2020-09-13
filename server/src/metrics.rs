use prometheus::{
    Encoder, Gauge, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::{
    convert::TryInto,
    error::Error,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error};

type RegistryFn = Box<dyn FnOnce(&Registry) -> Result<(), prometheus::Error>>;

pub struct PlayerMetrics {
    pub players_connected: IntCounter,
    pub players_disconnected: IntCounterVec, // timeout, network_error, gracefully
}

pub struct NetworkRequestMetrics {
    pub chunks_request_dropped: IntCounter,
    pub chunks_served_from_cache: IntCounter,
    pub chunks_generation_triggered: IntCounter,
}

pub struct ChunkGenMetrics {
    pub chunks_requested: IntCounter,
    pub chunks_served: IntCounter,
    pub chunks_canceled: IntCounter,
}

pub struct TickMetrics {
    pub chonks_count: IntGauge,
    pub chunks_count: IntGauge,
    pub entity_count: IntGauge,
    pub tick_time: IntGaugeVec,
    pub build_info: IntGauge,
    pub start_time: IntGauge,
    pub time_of_day: Gauge,
    pub light_count: IntGauge,
    tick: Arc<AtomicU64>,
}

pub struct ServerMetrics {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    registry: Option<Registry>,
    tick: Arc<AtomicU64>,
}

impl PlayerMetrics {
    pub fn new() -> Result<(Self, RegistryFn), prometheus::Error> {
        let players_connected = IntCounter::with_opts(Opts::new(
            "players_connected",
            "shows the number of clients joined to the server",
        ))?;
        let players_disconnected = IntCounterVec::new(
            Opts::new(
                "players_disconnected",
                "shows the number of clients disconnected from the server and the reason",
            ),
            &["reason"],
        )?;

        let players_connected_clone = players_connected.clone();
        let players_disconnected_clone = players_disconnected.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(players_connected_clone))?;
            registry.register(Box::new(players_disconnected_clone))?;
            Ok(())
        };

        Ok((
            Self {
                players_connected,
                players_disconnected,
            },
            Box::new(f),
        ))
    }
}

impl NetworkRequestMetrics {
    pub fn new() -> Result<(Self, RegistryFn), prometheus::Error> {
        let chunks_request_dropped = IntCounter::with_opts(Opts::new(
            "chunks_request_dropped",
            "number of all chunk request dropped, e.g because the player was to far away",
        ))?;
        let chunks_served_from_cache = IntCounter::with_opts(Opts::new(
            "chunks_served_from_cache",
            "number of all requested chunks already generated and could be served out of cache",
        ))?;
        let chunks_generation_triggered = IntCounter::with_opts(Opts::new(
            "chunks_generation_triggered",
            "number of all chunks that were requested and needs to be generated",
        ))?;

        let chunks_request_dropped_clone = chunks_request_dropped.clone();
        let chunks_served_from_cache_clone = chunks_served_from_cache.clone();
        let chunks_generation_triggered_clone = chunks_generation_triggered.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(chunks_request_dropped_clone))?;
            registry.register(Box::new(chunks_served_from_cache_clone))?;
            registry.register(Box::new(chunks_generation_triggered_clone))?;
            Ok(())
        };

        Ok((
            Self {
                chunks_request_dropped,
                chunks_served_from_cache,
                chunks_generation_triggered,
            },
            Box::new(f),
        ))
    }
}

impl ChunkGenMetrics {
    pub fn new() -> Result<(Self, RegistryFn), prometheus::Error> {
        let chunks_requested = IntCounter::with_opts(Opts::new(
            "chunks_requested",
            "number of all chunks requested on the server",
        ))?;
        let chunks_served = IntCounter::with_opts(Opts::new(
            "chunks_served",
            "number of all requested chunks already served on the server",
        ))?;
        let chunks_canceled = IntCounter::with_opts(Opts::new(
            "chunks_canceled",
            "number of all canceled chunks on the server",
        ))?;

        let chunks_requested_clone = chunks_requested.clone();
        let chunks_served_clone = chunks_served.clone();
        let chunks_canceled_clone = chunks_canceled.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(chunks_requested_clone))?;
            registry.register(Box::new(chunks_served_clone))?;
            registry.register(Box::new(chunks_canceled_clone))?;
            Ok(())
        };

        Ok((
            Self {
                chunks_requested,
                chunks_served,
                chunks_canceled,
            },
            Box::new(f),
        ))
    }
}

impl TickMetrics {
    pub fn new(tick: Arc<AtomicU64>) -> Result<(Self, RegistryFn), Box<dyn Error>> {
        let chonks_count = IntGauge::with_opts(Opts::new(
            "chonks_count",
            "number of all chonks currently active on the server",
        ))?;
        let chunks_count = IntGauge::with_opts(Opts::new(
            "chunks_count",
            "number of all chunks currently active on the server",
        ))?;
        let entity_count = IntGauge::with_opts(Opts::new(
            "entity_count",
            "number of all entities currently active on the server",
        ))?;
        let opts = Opts::new("veloren_build_info", "Build information")
            .const_label("hash", &common::util::GIT_HASH)
            .const_label("version", "");
        let build_info = IntGauge::with_opts(opts)?;
        let start_time = IntGauge::with_opts(Opts::new(
            "veloren_start_time",
            "start time of the server in seconds since EPOCH",
        ))?;
        let time_of_day =
            Gauge::with_opts(Opts::new("time_of_day", "ingame time in ingame-seconds"))?;
        let light_count = IntGauge::with_opts(Opts::new(
            "light_count",
            "number of all lights currently active on the server",
        ))?;
        let tick_time = IntGaugeVec::new(
            Opts::new("tick_time", "time in ns required for a tick of the server"),
            &["period"],
        )?;

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        start_time.set(since_the_epoch.as_secs().try_into()?);

        let chonks_count_clone = chonks_count.clone();
        let chunks_count_clone = chunks_count.clone();
        let entity_count_clone = entity_count.clone();
        let build_info_clone = build_info.clone();
        let start_time_clone = start_time.clone();
        let time_of_day_clone = time_of_day.clone();
        let light_count_clone = light_count.clone();
        let tick_time_clone = tick_time.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(chonks_count_clone))?;
            registry.register(Box::new(chunks_count_clone))?;
            registry.register(Box::new(entity_count_clone))?;
            registry.register(Box::new(build_info_clone))?;
            registry.register(Box::new(start_time_clone))?;
            registry.register(Box::new(time_of_day_clone))?;
            registry.register(Box::new(light_count_clone))?;
            registry.register(Box::new(tick_time_clone))?;
            Ok(())
        };

        Ok((
            Self {
                chonks_count,
                chunks_count,
                entity_count,
                tick_time,
                build_info,
                start_time,
                time_of_day,
                light_count,
                tick,
            },
            Box::new(f),
        ))
    }

    pub fn is_100th_tick(&self) -> bool { self.tick.load(Ordering::Relaxed).rem_euclid(100) == 0 }
}

impl ServerMetrics {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        let running = Arc::new(AtomicBool::new(false));
        let tick = Arc::new(AtomicU64::new(0));
        let registry = Some(Registry::new());

        Self {
            running,
            handle: None,
            registry,
            tick,
        }
    }

    pub fn registry(&self) -> &Registry {
        match self.registry {
            Some(ref r) => r,
            None => panic!("You cannot longer register new metrics after the server has started!"),
        }
    }

    pub fn run(&mut self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
        self.running.store(true, Ordering::Relaxed);
        let running2 = self.running.clone();

        let registry = self
            .registry
            .take()
            .expect("ServerMetrics must be already started");

        //TODO: make this a job
        self.handle = Some(thread::spawn(move || {
            let server = tiny_http::Server::http(addr).unwrap();
            const TIMEOUT: Duration = Duration::from_secs(1);
            debug!("starting tiny_http server to serve metrics");
            while running2.load(Ordering::Relaxed) {
                let request = match server.recv_timeout(TIMEOUT) {
                    Ok(Some(rq)) => rq,
                    Ok(None) => continue,
                    Err(e) => {
                        error!(?e, "metrics http server error");
                        break;
                    },
                };
                let mf = registry.gather();
                let encoder = TextEncoder::new();
                let mut buffer = vec![];
                encoder
                    .encode(&mf, &mut buffer)
                    .expect("Failed to encoder metrics text.");
                let response = tiny_http::Response::from_string(
                    String::from_utf8(buffer).expect("Failed to parse bytes as a string."),
                );
                if let Err(e) = request.respond(response) {
                    error!(
                        ?e,
                        "The metrics HTTP server had encountered and error with answering",
                    );
                }
            }
            debug!("stopping tiny_http server to serve metrics");
        }));
        Ok(())
    }

    pub fn tick(&self) -> u64 { self.tick.fetch_add(1, Ordering::Relaxed) + 1 }

    pub fn tick_clone(&self) -> Arc<AtomicU64> { self.tick.clone() }
}

impl Drop for ServerMetrics {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let handle = self.handle.take();
        handle
            .expect("ServerMetrics worker handle does not exist.")
            .join()
            .expect("Error shutting down prometheus metric exporter");
    }
}
