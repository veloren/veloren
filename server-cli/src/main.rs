#![deny(unsafe_code)]

use common::clock::Clock;
use heaptrack::track_mem;
use log::info;
use server::{Event, Input, Server, ServerSettings};
use std::time::Duration;

track_mem!();

use std::sync::{mpsc, Arc};
use worldsim::{
    regionmanager::{RegionManager, meta::RegionManagerMsg},
    server::meta::{ServerMsg},
    job::JobManager,
    region::Region,
};
const TPS: u64 = 30;

fn main() {
    // Init logging
    pretty_env_logger::init();

    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::start();

    // Load settings
    let settings = ServerSettings::load();

    let (region_manager_tx, region_manager_rx) = mpsc::channel::<RegionManagerMsg>();
    let (server_tx, server_rx) = mpsc::channel::<ServerMsg>();

    let mut region_manager = RegionManager::new(region_manager_tx, server_rx);
    let mut job_manager: Arc<JobManager> = Arc::new(JobManager::new());
    let mut server = worldsim::server::Server::new(server_tx,region_manager_rx,job_manager.clone());
    let mut region = Region::new((0,0),job_manager.clone());

    job_manager.repeat(move || region_manager.work() );
    job_manager.repeat(move || server.work() );

    // Create server
    let mut server = Server::new(settings).expect("Failed to create server instance!");

    loop {
        let events = server
            .tick(Input::default(), clock.get_last_delta())
            .expect("Failed to tick server");

        for event in events {
            match event {
                Event::ClientConnected { entity: _ } => info!("Client connected!"),
                Event::ClientDisconnected { entity: _ } => info!("Client disconnected!"),
                Event::Chat { entity: _, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick.
        server.cleanup();

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
