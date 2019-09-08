extern crate prometheus;
extern crate prometheus_static_metric;
extern crate rouille;
use prometheus::{Encoder, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder};
use rouille::router;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;
use std::thread::JoinHandle;

pub struct ServerMetrics {
    pub chonks_count: IntGaugeVec,
    pub player_online: IntGauge,
    pub entity_count: IntGauge,
    pub tick_time: IntGaugeVec,
    pub build_info: IntGauge,
    pub light_count: IntGauge,
    pub registry: Registry,
    pub handle: Option<JoinHandle<()>>,
}

impl ServerMetrics {
    pub fn new() -> Self {
        let opts = Opts::new(
            "player_online",
            "shows the number of clients connected to the server",
        );
        let player_online = IntGauge::with_opts(opts).unwrap();
        let opts = Opts::new(
            "entity_count",
            "number of all entities currently active on the server",
        );
        let entity_count = IntGauge::with_opts(opts).unwrap();
        let opts = Opts::new("veloren_build_info", "Build information")
            .const_label("hash", common::util::GIT_HASH)
            .const_label("version", "");
        let build_info = IntGauge::with_opts(opts).unwrap();
        let opts = Opts::new(
            "light_count",
            "number of all lights currently active on the server",
        );
        let light_count = IntGauge::with_opts(opts).unwrap();
        let vec = IntGaugeVec::new(
            Opts::new(
                "chonks_count",
                "number of all chonks currently active on the server",
            ),
            &["type"],
        )
        .unwrap();
        let chonks_count: IntGaugeVec = IntGaugeVec::from(vec);
        let vec = IntGaugeVec::new(
            Opts::new("tick_time", "time in ns requiered for a tick of the server"),
            &["period"],
        )
        .unwrap();
        let tick_time = IntGaugeVec::from(vec);

        let registry = Registry::new();
        //registry.register(Box::new(chonks_count.clone())).unwrap();
        registry.register(Box::new(player_online.clone())).unwrap();
        registry.register(Box::new(entity_count.clone())).unwrap();
        registry.register(Box::new(build_info.clone())).unwrap();
        //registry.register(Box::new(light_count.clone())).unwrap();
        registry.register(Box::new(chonks_count.clone())).unwrap();
        registry.register(Box::new(tick_time.clone())).unwrap();
        prometheus::register(Box::new(player_online.clone())).unwrap();
        prometheus::register(Box::new(entity_count.clone())).unwrap();
        prometheus::register(Box::new(build_info.clone())).unwrap();
        //prometheus::register(Box::new(light_count.clone())).unwrap();
        prometheus::register(Box::new(chonks_count.clone())).unwrap();
        prometheus::register(Box::new(tick_time.clone())).unwrap();

        let mut metrics = Self {
            chonks_count,
            player_online,
            entity_count,
            tick_time,
            build_info,
            light_count,
            registry,
            handle: None,
        };

        let handle = thread::spawn(|| {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 14005);
            rouille::start_server(addr, move |request| {
                router!(request,
                        (GET) (/metrics) => {
                        let encoder = TextEncoder::new();
                        let mut buffer = vec![];
                        let mf = prometheus::gather();
                        encoder.encode(&mf, &mut buffer).unwrap();
                        rouille::Response::text(String::from_utf8(buffer).unwrap())
                },
                _ => rouille::Response::empty_404()
                )
            });
        });
        metrics.handle = Some(handle);

        metrics
    }
}
