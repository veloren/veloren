use world::World;
use std::net::{TcpListener, SocketAddr};
use std::thread;
use bifrost::Relay;
use network::event::NewSessionEvent;

pub fn init_network(relay: Relay<World>, world: &mut World, port: u16) -> bool {

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).unwrap();

    let handle = thread::spawn(move || {
        listen_for_connections(relay, listener);
    });

    println!("Server listening on port {}", port);
    true
}

fn listen_for_connections(relay: Relay<World>, listener: TcpListener) {

    let mut id = 0;

    for stream in listener.incoming() {
        relay.send(NewSessionEvent {
            session_id: id,
            stream: stream.unwrap(),
        });
        id += 1;
    }
}