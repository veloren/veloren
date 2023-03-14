use prometheus::{
    Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, Opts, Registry,
};
use std::{
    convert::TryInto,
    error::Error,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct PhysicsMetrics {
    pub entity_entity_collision_checks_count: IntCounter,
    pub entity_entity_collisions_count: IntCounter,
}

pub struct EcsSystemMetrics {
    // Gauges give us detailed information for random ticks
    pub system_start_time: IntGaugeVec,
    pub system_length_time: IntGaugeVec,
    pub system_thread_avg: GaugeVec,
    // Counter will only give us granularity on pool speed (2s?) for actual spike detection we
    // need the Historgram
    pub system_length_hist: HistogramVec,
    pub system_length_count: IntCounterVec,
}

pub struct PlayerMetrics {
    pub clients_connected: IntCounter,
    pub players_connected: IntCounter,
    pub clients_disconnected: IntCounterVec, // timeout, network_error, gracefully
}

pub struct NetworkRequestMetrics {
    pub chunks_request_dropped: IntCounter,
    pub chunks_served_from_memory: IntCounter,
    pub chunks_generation_triggered: IntCounter,
    pub chunks_served_lossy: IntCounter,
    pub chunks_served_lossless: IntCounter,
    pub chunks_serialisation_requests: IntCounter,
    pub chunks_distinct_serialisation_requests: IntCounter,
}

pub struct ChunkGenMetrics {
    pub chunks_requested: IntCounter,
    pub chunks_served: IntCounter,
    pub chunks_canceled: IntCounter,
}

pub struct JobMetrics {
    pub job_queried_hst: HistogramVec,
    pub job_execution_hst: HistogramVec,
}

pub struct TickMetrics {
    pub chonks_count: IntGauge,
    pub chunks_count: IntGauge,
    pub chunk_groups_count: IntGauge,
    pub entity_count: IntGauge,
    pub tick_time: IntGaugeVec,
    /// Timing of some subsections of `State::tick`.
    pub state_tick_time: IntGaugeVec,
    pub tick_time_hist: Histogram,
    pub build_info: IntGauge,
    pub start_time: IntGauge,
    pub time_of_day: Gauge,
    pub light_count: IntGauge,
}

pub struct ServerEventMetrics {
    pub event_count: IntCounterVec,
}

impl PhysicsMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let entity_entity_collision_checks_count = IntCounter::with_opts(Opts::new(
            "entity_entity_collision_checks_count",
            "shows the number of collision checks",
        ))?;
        let entity_entity_collisions_count = IntCounter::with_opts(Opts::new(
            "entity_entity_collisions_count",
            "shows the number of actual collisions detected",
        ))?;

        registry.register(Box::new(entity_entity_collision_checks_count.clone()))?;
        registry.register(Box::new(entity_entity_collisions_count.clone()))?;

        Ok(Self {
            entity_entity_collision_checks_count,
            entity_entity_collisions_count,
        })
    }
}

impl EcsSystemMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let bucket = vec![
            Duration::from_micros(1).as_secs_f64(),
            Duration::from_micros(10).as_secs_f64(),
            Duration::from_micros(100).as_secs_f64(),
            Duration::from_micros(200).as_secs_f64(),
            Duration::from_micros(400).as_secs_f64(),
            Duration::from_millis(2).as_secs_f64(),
            Duration::from_millis(5).as_secs_f64(),
            Duration::from_millis(10).as_secs_f64(),
            Duration::from_millis(20).as_secs_f64(),
            Duration::from_millis(30).as_secs_f64(),
            Duration::from_millis(50).as_secs_f64(),
            Duration::from_millis(100).as_secs_f64(),
        ];
        let system_length_hist = HistogramVec::new(
            HistogramOpts::new(
                "system_length_hist",
                "shows the detailed time in ns inside each ECS system as histogram",
            )
            .buckets(bucket),
            &["system"],
        )?;
        let system_length_count = IntCounterVec::new(
            Opts::new(
                "system_length_count",
                "shows the detailed time in ns inside each ECS system",
            ),
            &["system"],
        )?;
        let system_start_time = IntGaugeVec::new(
            Opts::new(
                "system_start_time",
                "start relative to tick start in ns required per ECS system",
            ),
            &["system"],
        )?;
        let system_length_time = IntGaugeVec::new(
            Opts::new("system_length_time", "time in ns required per ECS system"),
            &["system"],
        )?;
        let system_thread_avg = GaugeVec::new(
            Opts::new(
                "system_thread_avg",
                "average threads used by the ECS system",
            ),
            &["system"],
        )?;

        registry.register(Box::new(system_start_time.clone()))?;
        registry.register(Box::new(system_length_time.clone()))?;
        registry.register(Box::new(system_thread_avg.clone()))?;
        registry.register(Box::new(system_length_hist.clone()))?;
        registry.register(Box::new(system_length_count.clone()))?;

        Ok(Self {
            system_start_time,
            system_length_time,
            system_thread_avg,
            system_length_hist,
            system_length_count,
        })
    }
}

impl PlayerMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let clients_connected = IntCounter::with_opts(Opts::new(
            "clients_connected",
            "shows the number of clients joined to the server",
        ))?;
        let players_connected = IntCounter::with_opts(Opts::new(
            "players_connected",
            "shows the number of players joined to the server. A player is a client, that \
             registers itself. Bots are not players (but clients)",
        ))?;
        let clients_disconnected = IntCounterVec::new(
            Opts::new(
                "clients_disconnected",
                "shows the number of clients disconnected from the server and the reason",
            ),
            &["reason"],
        )?;

        registry.register(Box::new(clients_connected.clone()))?;
        registry.register(Box::new(players_connected.clone()))?;
        registry.register(Box::new(clients_disconnected.clone()))?;

        Ok(Self {
            clients_connected,
            players_connected,
            clients_disconnected,
        })
    }
}

impl NetworkRequestMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let chunks_request_dropped = IntCounter::with_opts(Opts::new(
            "chunks_request_dropped",
            "number of all chunk request dropped, e.g because the player was to far away",
        ))?;
        let chunks_served_from_memory = IntCounter::with_opts(Opts::new(
            "chunks_served_from_memory",
            "number of all requested chunks already generated and could be served out of cache",
        ))?;
        let chunks_generation_triggered = IntCounter::with_opts(Opts::new(
            "chunks_generation_triggered",
            "number of all chunks that were requested and needs to be generated",
        ))?;
        let chunks_served_lossy = IntCounter::with_opts(Opts::new(
            "chunks_served_lossy",
            "number of chunks that were sent with lossy compression requested",
        ))?;
        let chunks_served_lossless = IntCounter::with_opts(Opts::new(
            "chunks_served_lossless",
            "number of chunks that were sent with lossless compression requested",
        ))?;
        let chunks_serialisation_requests = IntCounter::with_opts(Opts::new(
            "chunks_serialisation_requests",
            "number of requests for the sys chunk_serialisation",
        ))?;
        let chunks_distinct_serialisation_requests = IntCounter::with_opts(Opts::new(
            "chunks_distinct_serialisation_requests",
            "number of distinct chunks in requests for the sys chunk_serialisation",
        ))?;

        registry.register(Box::new(chunks_request_dropped.clone()))?;
        registry.register(Box::new(chunks_served_from_memory.clone()))?;
        registry.register(Box::new(chunks_generation_triggered.clone()))?;
        registry.register(Box::new(chunks_served_lossy.clone()))?;
        registry.register(Box::new(chunks_served_lossless.clone()))?;
        registry.register(Box::new(chunks_serialisation_requests.clone()))?;
        registry.register(Box::new(chunks_distinct_serialisation_requests.clone()))?;

        Ok(Self {
            chunks_request_dropped,
            chunks_served_from_memory,
            chunks_generation_triggered,
            chunks_served_lossy,
            chunks_served_lossless,
            chunks_serialisation_requests,
            chunks_distinct_serialisation_requests,
        })
    }
}

impl ChunkGenMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
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

        registry.register(Box::new(chunks_requested.clone()))?;
        registry.register(Box::new(chunks_served.clone()))?;
        registry.register(Box::new(chunks_canceled.clone()))?;

        Ok(Self {
            chunks_requested,
            chunks_served,
            chunks_canceled,
        })
    }
}

impl JobMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let bucket = vec![
            Duration::from_micros(100).as_secs_f64(),
            Duration::from_millis(2).as_secs_f64(),
            Duration::from_millis(100).as_secs_f64(),
        ];

        let job_queried_hst = HistogramVec::new(
            HistogramOpts::new(
                "job_queried_hst",
                "shows the detailed time each job name took from query till it started to execute \
                 as histogram",
            )
            .buckets(bucket),
            &["name"],
        )?;

        let bucket = vec![
            Duration::from_millis(5).as_secs_f64(),
            Duration::from_millis(20).as_secs_f64(),
            Duration::from_millis(50).as_secs_f64(),
            Duration::from_millis(100).as_secs_f64(),
            Duration::from_millis(200).as_secs_f64(),
            Duration::from_millis(500).as_secs_f64(),
            Duration::from_millis(1000).as_secs_f64(),
            Duration::from_millis(10000).as_secs_f64(),
        ];

        let job_execution_hst = HistogramVec::new(
            HistogramOpts::new(
                "job_execution_hst",
                "shows the detailed time each job name took from start of execution until it \
                 finished as histogram",
            )
            .buckets(bucket),
            &["name"],
        )?;

        registry.register(Box::new(job_queried_hst.clone()))?;
        registry.register(Box::new(job_execution_hst.clone()))?;

        Ok(Self {
            job_queried_hst,
            job_execution_hst,
        })
    }
}

impl TickMetrics {
    pub fn new(registry: &Registry) -> Result<Self, Box<dyn Error>> {
        let chonks_count = IntGauge::with_opts(Opts::new(
            "chonks_count",
            "number of all chonks currently active on the server",
        ))?;
        let chunks_count = IntGauge::with_opts(Opts::new(
            "chunks_count",
            "number of all chunks currently active on the server",
        ))?;
        let chunk_groups_count = IntGauge::with_opts(Opts::new(
            "chunk_groups_count",
            "number of 4×4×4 groups currently allocated by chunks on the server",
        ))?;
        let entity_count = IntGauge::with_opts(Opts::new(
            "entity_count",
            "number of all entities currently active on the server",
        ))?;
        let opts = Opts::new("veloren_build_info", "Build information")
            .const_label("hash", *common::util::GIT_HASH)
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
        let state_tick_time = IntGaugeVec::new(
            Opts::new(
                "state_tick_time",
                "time in ns for some subsections of State::tick",
            ),
            &["period"],
        )?;
        // 33.33ms is the ideal tick time. So we have hight detail around it.
        // 300/700 are to detect high I/O blocks
        let bucket = vec![
            Duration::from_millis(8).as_secs_f64(),
            Duration::from_millis(16).as_secs_f64(),
            Duration::from_millis(24).as_secs_f64(),
            Duration::from_millis(30).as_secs_f64(),
            Duration::from_millis(33).as_secs_f64(),
            Duration::from_millis(37).as_secs_f64(),
            Duration::from_millis(45).as_secs_f64(),
            Duration::from_millis(60).as_secs_f64(),
            Duration::from_millis(100).as_secs_f64(),
            Duration::from_millis(300).as_secs_f64(),
            Duration::from_millis(700).as_secs_f64(),
        ];
        let tick_time_hist = Histogram::with_opts(
            HistogramOpts::new(
                "tick_time_hist",
                "shows the detailed time in ns spend for the whole tick as histogram",
            )
            .buckets(bucket),
        )?;

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        start_time.set(since_the_epoch.as_secs().try_into()?);

        registry.register(Box::new(chonks_count.clone()))?;
        registry.register(Box::new(chunks_count.clone()))?;
        registry.register(Box::new(chunk_groups_count.clone()))?;
        registry.register(Box::new(entity_count.clone()))?;
        registry.register(Box::new(build_info.clone()))?;
        registry.register(Box::new(start_time.clone()))?;
        registry.register(Box::new(time_of_day.clone()))?;
        registry.register(Box::new(light_count.clone()))?;
        registry.register(Box::new(tick_time.clone()))?;
        registry.register(Box::new(state_tick_time.clone()))?;
        registry.register(Box::new(tick_time_hist.clone()))?;

        Ok(Self {
            chonks_count,
            chunks_count,
            chunk_groups_count,
            entity_count,
            tick_time,
            state_tick_time,
            tick_time_hist,
            build_info,
            start_time,
            time_of_day,
            light_count,
        })
    }
}

impl ServerEventMetrics {
    pub fn new(registry: &Registry) -> Result<Self, Box<dyn Error>> {
        let event_count = IntCounterVec::new(
            Opts::new("event_count", "number of ServerEvents handled"),
            &["event"],
        )?;
        registry.register(Box::new(event_count.clone()))?;

        Ok(Self { event_count })
    }
}
