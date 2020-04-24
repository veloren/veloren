use prometheus::{Encoder, Gauge, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder};
use rouille::{router, Server};
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

pub struct SimpleMetrics {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    registry: Option<Registry>,
}

impl SimpleMetrics {
    pub fn new() -> Self {
        let running = Arc::new(AtomicBool::new(false));
        let registry = Some(Registry::new());

        Self {
            running,
            handle: None,
            registry,
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
            while running2.load(Ordering::Relaxed) {
                server.poll();
                // Poll at 10Hz
                thread::sleep(Duration::from_millis(100));
            }
        }));
        Ok(())
    }
}

impl Drop for SimpleMetrics {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let handle = self.handle.take();
        handle
            .expect("ServerMetrics worker handle does not exist.")
            .join()
            .expect("Error shutting down prometheus metric exporter");
    }
}