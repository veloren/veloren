extern crate prometheus;
extern crate hyper;
extern crate prometheus_static_metric;
use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};
use prometheus::{TextEncoder, Encoder, Registry, Counter, Gauge, Opts, GaugeVec, CounterVec};
use prometheus_static_metric::make_static_metric;
use std::thread;
use std::time::Duration;

make_static_metric! {
    pub struct StaticChonkGaugeVec: Gauge {
        "method" => {
            hetero,
            hash,
            homo,
        },
    }
}
pub struct ServerMetrics {
    pub chonks_count: StaticChonkGaugeVec,
    pub player_online:  Gauge,
    pub entity_count: Gauge,
    pub tick_time: Gauge,
    pub build_info: Gauge,
    pub light_count: Gauge,
    pub registry: Registry,
}

fn metric_service(_req: Request<Body>) -> Response<Body> {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    let mf = prometheus::gather();
    encoder.encode(&mf, &mut buffer).unwrap();
    Response::builder()
        .header(hyper::header::CONTENT_TYPE, encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
}

impl ServerMetrics {
    fn metric_service(&self, _req: Request<Body>) -> Response<Body> {
        let encoder = TextEncoder::new();
        let mut buffer = vec![];
        let mf = self.registry.gather();
        encoder.encode(&mf, &mut buffer).unwrap();
        Response::builder()
            .header(hyper::header::CONTENT_TYPE, encoder.format_type())
            .body(Body::from(buffer))
            .unwrap()
    }

    pub fn new() -> Self {
        let opts = Opts::new("player_online", "shows the number of clients connected to the server");
        let player_online = Gauge::with_opts(opts).unwrap();
        let opts = Opts::new("entity_count", "number of all entities currently active on the server");
        let entity_count = Gauge::with_opts(opts).unwrap();
        let opts = Opts::new("tick_time", "time in ms requiered for a tick of the server");
        let tick_time = Gauge::with_opts(opts).unwrap();
        let opts = Opts::new("veloren_build_info", "Build information")
            .const_label("hash", common::util::GIT_HASH)
            .const_label("version", "");
        let build_info = Gauge::with_opts(opts).unwrap();
        let opts = Opts::new("light_count", "number of all lights currently active on the server");
        let light_count = Gauge::with_opts(opts).unwrap();
        let vec = GaugeVec::new(Opts::new("chonks_count", "number of all chonks currently active on the server"), &["method"]).unwrap();
        let chonks_count = StaticChonkGaugeVec::from(&vec);

        chonks_count.hetero.set(1337.0);
        chonks_count.hash.set(42.0);
        entity_count.set(42.0);

        let registry = Registry::new();
        //registry.register(Box::new(chonks_count.clone())).unwrap();
        registry.register(Box::new(player_online.clone())).unwrap();
        registry.register(Box::new(entity_count.clone())).unwrap();
        registry.register(Box::new(tick_time.clone())).unwrap();
        registry.register(Box::new(build_info.clone())).unwrap();
        registry.register(Box::new(light_count.clone())).unwrap();
        prometheus::register(Box::new(player_online.clone())).unwrap();
        prometheus::register(Box::new(entity_count.clone())).unwrap();
        prometheus::register(Box::new(tick_time.clone())).unwrap();
        prometheus::register(Box::new(build_info.clone())).unwrap();
        prometheus::register(Box::new(light_count.clone())).unwrap();

        let mut metrics = Self{
            chonks_count,
            player_online,
            entity_count,
            tick_time,
            build_info,
            light_count,
            registry,
        };

        let addr = ([0, 0, 0, 0], 14005).into();
        let service = || service_fn_ok(metric_service);
        let server = Server::bind(&addr)
            .serve(service)
            .map_err(|e| panic!("{}", e));

        let handle = thread::spawn(|| {
            hyper::rt::run(server);
        });

        metrics
    }
}