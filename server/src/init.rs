// Standard
use std::time::Duration;

// Library
use bifrost::{Relay, event};
use config::load_config;

// Project
//use common::logger::ConsoleLogger;

// Local
use network::init::init_network;
use server_context::{update_world, ServerContext, WORLD_UPDATE_TICK};


pub fn init_server(relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
    let config = load_config();

    init_network(relay.clone(), ctx, config.network.port);

    relay.schedule(event(update_world), Duration::from_millis(WORLD_UPDATE_TICK));

    info!("Server started");
}
