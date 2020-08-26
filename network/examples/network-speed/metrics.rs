use prometheus::{Encoder, Registry, TextEncoder};
use std::{
    error::Error,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
use tiny_http;
use tracing::*;

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
            let server = tiny_http::Server::http(addr).unwrap();
            const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
            debug!("starting tiny_http server to serve metrics");
            while running2.load(Ordering::Relaxed) {
                let request = match server.recv_timeout(TIMEOUT) {
                    Ok(Some(rq)) => rq,
                    Ok(None) => continue,
                    Err(e) => {
                        println!("Error: {}", e);
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
                match request.respond(response) {
                    Err(e) => error!(
                        ?e,
                        "The metrics HTTP server had encountered and error with answering"
                    ),
                    _ => (),
                }
            }
            debug!("Stopping tiny_http server to serve metrics");
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
