#![feature(
    euclidean_division,
    duration_float,
    trait_alias,
    bind_by_move_pattern_guards
)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

pub mod assets;
pub mod clock;
pub mod comp;
pub mod figure;
pub mod msg;
pub mod ray;
pub mod state;
pub mod sys;
pub mod terrain;
pub mod util;
pub mod vol;
pub mod volumes;

// TODO: unignore the code here, for some reason it refuses to compile here while has no problems copy-pasted elsewhere
/// The networking module containing high-level wrappers of `TcpListener` and `TcpStream` (`PostOffice` and `PostBox` respectively) and data types used by both the server and client
/// # Examples
/// ```ignore
/// use std::net::SocketAddr;
/// use veloren_common::net::{PostOffice, PostBox};
///
/// let listen_addr = SocketAddr::from(([0, 0, 0, 0], 12345u16));
/// let conn_addr = SocketAddr::from(([127, 0, 0, 1], 12345u16));
///
/// let server: PostOffice<String, String> = PostOffice::new(&listen_addr).unwrap();
/// let client: PostBox<String, String> = PostBox::to_server(&conn_addr).unwrap();
/// std::thread::sleep(std::time::Duration::from_millis(100));
///
/// let scon = server.get_iter().unwrap().next().unwrap().unwrap();
/// std::thread::sleep(std::time::Duration::from_millis(100));
///
/// scon.send(String::from("foo"));
/// client.send(String::from("bar"));
/// std::thread::sleep(std::time::Duration::from_millis(100));
///
/// assert_eq!("foo", client.recv_iter().unwrap().next().unwrap().unwrap());
/// assert_eq!("bar", scon.recv_iter().unwrap().next().unwrap().unwrap());
/// ```
pub mod net;
