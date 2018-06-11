extern crate server;

extern crate pretty_env_logger;
#[macro_use] extern crate log;
use server::Server;

fn main() {
    pretty_env_logger::init();

    info!("Server starting...");

    let server = Server::new();
    server.start();
}
