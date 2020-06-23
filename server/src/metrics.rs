use prometheus::{Encoder, Gauge, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder};
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

pub struct TickMetrics {
    pub chonks_count: IntGauge,
    pub chunks_count: IntGauge,
    pub player_online: IntGauge,
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

impl TickMetrics {
    #[allow(clippy::useless_conversion)] // TODO: Pending review in #587
    pub fn new(registry: &Registry, tick: Arc<AtomicU64>) -> Result<Self, Box<dyn Error>> {
        let player_online = IntGauge::with_opts(Opts::new(
            "player_online",
            "shows the number of clients connected to the server",
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
        let chonks_count = IntGauge::with_opts(Opts::new(
            "chonks_count",
            "number of all chonks currently active on the server",
        ))?;
        let chunks_count = IntGauge::with_opts(Opts::new(
            "chunks_count",
            "number of all chunks currently active on the server",
        ))?;
        let tick_time = IntGaugeVec::from(IntGaugeVec::new(
            Opts::new("tick_time", "time in ns requiered for a tick of the server"),
            &["period"],
        )?);

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        start_time.set(since_the_epoch.as_secs().try_into()?);

        registry.register(Box::new(player_online.clone()))?;
        registry.register(Box::new(entity_count.clone()))?;
        registry.register(Box::new(build_info.clone()))?;
        registry.register(Box::new(start_time.clone()))?;
        registry.register(Box::new(time_of_day.clone()))?;
        registry.register(Box::new(chonks_count.clone()))?;
        registry.register(Box::new(chunks_count.clone()))?;
        registry.register(Box::new(tick_time.clone()))?;

        Ok(Self {
            chonks_count,
            chunks_count,
            player_online,
            entity_count,
            tick_time,
            build_info,
            start_time,
            time_of_day,
            light_count,
            tick,
        })
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
