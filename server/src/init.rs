use bifrost::{Relay, event};
use config::load_config;
use network::init::init_network;
use std::time::Duration;
use server_context::{update_world, ServerContext, WORLD_UPDATE_TICK};
//use common::logger::ConsoleLogger;


pub fn init_server(relay: &Relay<ServerContext>, ctx: &mut ServerContext) {
    let config = load_config();

    init_network(relay.clone(), ctx, config.network.port);

    relay.schedule(event(update_world), Duration::from_millis(WORLD_UPDATE_TICK));

    info!("Server started");
}
