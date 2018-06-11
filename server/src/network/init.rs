use server_context::ServerContext;
use std::net::{TcpListener, SocketAddr};
use std::thread;
use bifrost::Relay;
use network::event::NewSessionEvent;
use std::net::TcpStream;
use std::io::Error;

pub fn init_network(relay: Relay<ServerContext>, world: &mut ServerContext, port: u16) -> bool {

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).unwrap();

    let _handle = thread::spawn(move || {
        listen_for_connections(relay, listener);
    });

    info!("Server listening on port {}", port);
    true
}

fn listen_for_connections(relay: Relay<ServerContext>, listener: TcpListener) {

    let mut id = 0;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                match handle_new_connection(&relay, stream, id) {
                    Ok(_) => id += 1,
                    Err(e) => error!("New connection error : {}", e),
                }
            },
            Err(e) => error!("New connection error : {}", e),
        }
    }
}

fn handle_new_connection(relay: &Relay<ServerContext>, stream: TcpStream, id: u32) -> Result<(), Error> {
    stream.set_nodelay(true)?;
    relay.send(NewSessionEvent {
        session_id: id,
        stream,
    });
    Ok(())
}
