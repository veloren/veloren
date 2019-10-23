use log::info;
use prometheus::{Encoder, Gauge, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder};
use rouille::{router, Server};
use std::error::Error;
use std::{
    convert::TryInto,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct ServerMetrics {
    pub chonks_count: IntGauge,
    pub chunks_count: IntGauge,
    pub player_online: IntGauge,
    pub entity_count: IntGauge,
    pub tick_time: IntGaugeVec,
    pub build_info: IntGauge,
    pub start_time: IntGauge,
    pub time_of_day: Gauge,
    pub light_count: IntGauge,
    running: Arc<AtomicBool>,
    pub handle: Option<thread::JoinHandle<()>>,
    pub every_100th: i8,
}

impl ServerMetrics {
    pub fn new(addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        let opts = Opts::new(
            "player_online",
            "shows the number of clients connected to the server",
        );
        let player_online = IntGauge::with_opts(opts)?;
        let opts = Opts::new(
            "entity_count",
            "number of all entities currently active on the server",
        );
        let entity_count = IntGauge::with_opts(opts)?;
        let opts = Opts::new("veloren_build_info", "Build information")
            .const_label("hash", &common::util::GIT_HASH)
            .const_label("version", "");
        let build_info = IntGauge::with_opts(opts)?;
        let opts = Opts::new(
            "veloren_start_time",
            "start time of the server in seconds since EPOCH",
        );
        let start_time = IntGauge::with_opts(opts)?;
        let opts = Opts::new("time_of_day", "ingame time in ingame-seconds");
        let time_of_day = Gauge::with_opts(opts)?;
        let opts = Opts::new(
            "light_count",
            "number of all lights currently active on the server",
        );
        let light_count = IntGauge::with_opts(opts)?;
        let opts = Opts::new(
            "chonks_count",
            "number of all chonks currently active on the server",
        );
        let chonks_count = IntGauge::with_opts(opts)?;
        let opts = Opts::new(
            "chunks_count",
            "number of all chunks currently active on the server",
        );
        let chunks_count = IntGauge::with_opts(opts)?;
        let vec = IntGaugeVec::new(
            Opts::new("tick_time", "time in ns requiered for a tick of the server"),
            &["period"],
        )?;
        let tick_time = IntGaugeVec::from(vec);

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        start_time.set(since_the_epoch.as_secs().try_into()?);

        let registry = Registry::new();
        //registry.register(Box::new(chonks_count.clone())).unwrap();
        registry.register(Box::new(player_online.clone()))?;
        registry.register(Box::new(entity_count.clone()))?;
        registry.register(Box::new(build_info.clone()))?;
        registry.register(Box::new(start_time.clone()))?;
        registry.register(Box::new(time_of_day.clone()))?;
        //registry.register(Box::new(light_count.clone())).unwrap();
        registry.register(Box::new(chonks_count.clone()))?;
        registry.register(Box::new(chunks_count.clone()))?;
        registry.register(Box::new(tick_time.clone()))?;

        let running = Arc::new(AtomicBool::new(true));
        let running2 = running.clone();

        //TODO: make this a job
        let handle = Some(thread::spawn(move || {
            let server = Server::new(addr, move |request| {
                router!(request,
                        (GET) (/metrics) => {
                        let encoder = TextEncoder::new();
                        let mut buffer = vec![];
                        let mf = registry.gather();
                        encoder.encode(&mf, &mut buffer).expect("Failed to encoder metrics text.");
                        rouille::Response::text(String::from_utf8(buffer).expect("Failed to parse bytes as a string."))
                },
                _ => rouille::Response::empty_404()
                )
            })
            .expect("Failed to start server");
            info!("Started server metrics: {}", addr);
            while running2.load(Ordering::Relaxed) {
                server.poll();
                // Poll at 10Hz
                thread::sleep(Duration::from_millis(100));
            }
        }));

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
            running,
            handle,
            every_100th: 0,
        })
    }

    pub fn is_100th_tick(&mut self) -> bool {
        self.every_100th += 1;
        if self.every_100th == 100 {
            self.every_100th = 0;
            true
        } else {
            false
        }
    }
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
