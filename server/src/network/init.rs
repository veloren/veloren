// Standard
use std::net::{TcpListener, SocketAddr};
use std::thread;
use std::net::TcpStream;
use std::io::Error;

// Library
use bifrost::Relay;

// Project
use common::net::Conn;

// Local
use network::event::NewSessionEvent;
use server_context::ServerContext;

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
            Ok(stream) => match Conn::from_stream(stream) {
                Ok(conn) => match handle_new_connection(&relay, conn, id) {
                    Ok(_) => id += 1,
                    Err(e) => error!("New connection error : {}", e),
                },
                Err(e) => error!("Connection creation error: {:?}", e),
            },
            Err(e) => error!("New connection error : {}", e),
        }
    }
}

fn handle_new_connection(relay: &Relay<ServerContext>, conn: Conn, id: u32) -> Result<(), Error> {
    relay.send(NewSessionEvent {
        session_id: id,
        conn,
    });
    Ok(())
}
