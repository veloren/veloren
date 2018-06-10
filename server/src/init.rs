use bifrost::event::event;
use bifrost::Relay;
use config::load_config;
use network::init::init_network;
use std::time::Duration;
use world::{update_world, World, WORLD_UPDATE_TICK};


pub fn init_server(relay: &Relay<World>, ctx: &mut World) {
    let config = load_config();

    init_network(relay.clone(), ctx, config.network.port);

    relay.schedule(event(update_world), Duration::from_millis(WORLD_UPDATE_TICK));


    println!("Server started");
}
