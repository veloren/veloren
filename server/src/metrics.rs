extern crate hyper;
extern crate prometheus;
extern crate prometheus_static_metric;
use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};
use prometheus::{Encoder, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder};
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
    /*
    fn metric_service(&self, _req: Request<Body>) -> Response<Body> {
        let encoder = TextEncoder::new();
        let mut buffer = vec![];
        let mf = self.registry.gather();
        encoder.encode(&mf, &mut buffer).unwrap();
        Response::builder()
            .header(hyper::header::CONTENT_TYPE, encoder.format_type())
            .body(Body::from(buffer))
            .unwrap()
    }*/

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

        let addr = ([0, 0, 0, 0], 14005).into();
        let service = || service_fn_ok(metric_service);
        let server = Server::bind(&addr)
            .serve(service)
            .map_err(|e| panic!("{}", e));

        let handle = thread::spawn(|| {
            hyper::rt::run(server);
        });
        metrics.handle = Some(handle);

        metrics
    }
}
