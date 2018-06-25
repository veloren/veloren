use std::net::TcpStream;

pub struct Session {
}

impl Session {
    pub fn new(stream: TcpStream) -> Session {
        stream.set_nonblocking(true).unwrap(); // quickfix for client
        Session {
        }
    }

}
