use prometheus::{
    Gauge, GaugeVec, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
    Opts, Registry,
};
use std::{
    convert::TryInto,
    error::Error,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type RegistryFn = Box<dyn FnOnce(&Registry) -> Result<(), prometheus::Error>>;

pub struct PhysicsMetrics {
    pub entity_entity_collision_checks_count: IntCounter,
    pub entity_entity_collisions_count: IntCounter,
}

pub struct StateTickMetrics {
    // Counter will only give us granularity on pool speed (2s?) for actuall spike detection we
    // need the Historgram
    pub state_tick_time_hist: HistogramVec,
    pub state_tick_time_count: IntCounterVec,
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
}

pub struct ChunkGenMetrics {
    pub chunks_requested: IntCounter,
    pub chunks_served: IntCounter,
    pub chunks_canceled: IntCounter,
}

pub struct TickMetrics {
    pub chonks_count: IntGauge,
    pub chunks_count: IntGauge,
    pub chunk_groups_count: IntGauge,
    pub entity_count: IntGauge,
    pub tick_time: IntGaugeVec,
    pub build_info: IntGauge,
    pub start_time: IntGauge,
    pub time_of_day: Gauge,
    pub light_count: IntGauge,
    pub system_start_time: IntGaugeVec,
    pub system_length_time: IntGaugeVec,
    pub system_thread_avg: GaugeVec,
    tick: Arc<AtomicU64>,
}

impl PhysicsMetrics {
    pub fn new() -> Result<(Self, RegistryFn), prometheus::Error> {
        let entity_entity_collision_checks_count = IntCounter::with_opts(Opts::new(
            "entity_entity_collision_checks_count",
            "shows the number of collision checks",
        ))?;
        let entity_entity_collisions_count = IntCounter::with_opts(Opts::new(
            "entity_entity_collisions_count",
            "shows the number of actual collisions detected",
        ))?;

        let entity_entity_collision_checks_count_clone =
            entity_entity_collision_checks_count.clone();
        let entity_entity_collisions_count_clone = entity_entity_collisions_count.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(entity_entity_collision_checks_count_clone))?;
            registry.register(Box::new(entity_entity_collisions_count_clone))?;
            Ok(())
        };

        Ok((
            Self {
                entity_entity_collision_checks_count,
                entity_entity_collisions_count,
            },
            Box::new(f),
        ))
    }
}

impl StateTickMetrics {
    pub fn new() -> Result<(Self, RegistryFn), prometheus::Error> {
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
        let state_tick_time_hist = HistogramVec::new(
            HistogramOpts::new(
                "state_tick_time_hist",
                "shows the number of clients joined to the server",
            )
            .buckets(bucket),
            &["system"],
        )?;
        let state_tick_time_count = IntCounterVec::new(
            Opts::new(
                "state_tick_time_count",
                "shows the detailed time inside the `state_tick` for each system",
            ),
            &["system"],
        )?;

        let state_tick_time_hist_clone = state_tick_time_hist.clone();
        let state_tick_time_count_clone = state_tick_time_count.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(state_tick_time_hist_clone))?;
            registry.register(Box::new(state_tick_time_count_clone))?;
            Ok(())
        };

        Ok((
            Self {
                state_tick_time_hist,
                state_tick_time_count,
            },
            Box::new(f),
        ))
    }
}

impl PlayerMetrics {
    pub fn new() -> Result<(Self, RegistryFn), prometheus::Error> {
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

        let clients_connected_clone = clients_connected.clone();
        let players_connected_clone = players_connected.clone();
        let clients_disconnected_clone = clients_disconnected.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(clients_connected_clone))?;
            registry.register(Box::new(players_connected_clone))?;
            registry.register(Box::new(clients_disconnected_clone))?;
            Ok(())
        };

        Ok((
            Self {
                clients_connected,
                players_connected,
                clients_disconnected,
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
        let chunks_served_from_memory = IntCounter::with_opts(Opts::new(
            "chunks_served_from_memory",
            "number of all requested chunks already generated and could be served out of cache",
        ))?;
        let chunks_generation_triggered = IntCounter::with_opts(Opts::new(
            "chunks_generation_triggered",
            "number of all chunks that were requested and needs to be generated",
        ))?;

        let chunks_request_dropped_clone = chunks_request_dropped.clone();
        let chunks_served_from_memory_clone = chunks_served_from_memory.clone();
        let chunks_generation_triggered_clone = chunks_generation_triggered.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(chunks_request_dropped_clone))?;
            registry.register(Box::new(chunks_served_from_memory_clone))?;
            registry.register(Box::new(chunks_generation_triggered_clone))?;
            Ok(())
        };

        Ok((
            Self {
                chunks_request_dropped,
                chunks_served_from_memory,
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
    pub fn new() -> Result<(Self, RegistryFn), Box<dyn Error>> {
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

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        start_time.set(since_the_epoch.as_secs().try_into()?);

        let chonks_count_clone = chonks_count.clone();
        let chunks_count_clone = chunks_count.clone();
        let chunk_groups_count_clone = chunk_groups_count.clone();
        let entity_count_clone = entity_count.clone();
        let build_info_clone = build_info.clone();
        let start_time_clone = start_time.clone();
        let time_of_day_clone = time_of_day.clone();
        let light_count_clone = light_count.clone();
        let tick_time_clone = tick_time.clone();
        let tick = Arc::new(AtomicU64::new(0));
        let system_start_time_clone = system_start_time.clone();
        let system_length_time_clone = system_length_time.clone();
        let system_thread_avg_clone = system_thread_avg.clone();

        let f = |registry: &Registry| {
            registry.register(Box::new(chonks_count_clone))?;
            registry.register(Box::new(chunks_count_clone))?;
            registry.register(Box::new(chunk_groups_count_clone))?;
            registry.register(Box::new(entity_count_clone))?;
            registry.register(Box::new(build_info_clone))?;
            registry.register(Box::new(start_time_clone))?;
            registry.register(Box::new(time_of_day_clone))?;
            registry.register(Box::new(light_count_clone))?;
            registry.register(Box::new(tick_time_clone))?;
            registry.register(Box::new(system_start_time_clone))?;
            registry.register(Box::new(system_length_time_clone))?;
            registry.register(Box::new(system_thread_avg_clone))?;
            Ok(())
        };

        Ok((
            Self {
                chonks_count,
                chunks_count,
                chunk_groups_count,
                entity_count,
                tick_time,
                build_info,
                start_time,
                time_of_day,
                light_count,
                system_start_time,
                system_length_time,
                system_thread_avg,
                tick,
            },
            Box::new(f),
        ))
    }

    pub fn tick(&self) { self.tick.fetch_add(1, Ordering::Relaxed); }

    pub fn is_100th_tick(&self) -> bool { self.tick.load(Ordering::Relaxed).rem_euclid(100) == 0 }
}
