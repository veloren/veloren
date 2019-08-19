#![deny(unsafe_code)]
#![type_length_limit = "1664759"]
#![feature(
    euclidean_division,
    duration_float,
    trait_alias,
    bind_by_move_pattern_guards,
    option_flattening, // Converts Option<Option<Item>> into Option<Item> TODO: Remove this once this feature becomes stable
)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

pub mod assets;
pub mod clock;
pub mod comp;
pub mod event;
pub mod figure;
pub mod msg;
pub mod npc;
pub mod ray;
pub mod state;
pub mod sys;
pub mod terrain;
pub mod util;
pub mod vol;
pub mod volumes;

/// The networking module containing high-level wrappers of `TcpListener` and `TcpStream` (`PostOffice` and `PostBox` respectively) and data types used by both the server and client.
/// # Examples
/// ```
/// use std::net::SocketAddr;
/// use veloren_common::net::{PostOffice, PostBox};
///
/// let listen_addr = SocketAddr::from(([0, 0, 0, 0], 12345u16));
/// let conn_addr = SocketAddr::from(([127, 0, 0, 1], 12345u16));
///
/// let mut server: PostOffice<String, String> = PostOffice::bind(listen_addr).unwrap();
/// let mut client: PostBox<String, String> = PostBox::to(conn_addr).unwrap();
/// std::thread::sleep(std::time::Duration::from_millis(100));
///
/// let mut scon = server.new_postboxes().next().unwrap();
/// std::thread::sleep(std::time::Duration::from_millis(100));
///
/// scon.send_message(String::from("foo"));
/// client.send_message(String::from("bar"));
/// std::thread::sleep(std::time::Duration::from_millis(100));
///
/// assert_eq!("foo", client.next_message().unwrap());
/// assert_eq!("bar", scon.next_message().unwrap());
/// ```
pub mod net;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatType {
    Broadcast,
    Chat,
    GameUpdate,
    Private,
    Tell,
    Say,
    Group,
    Faction,
    Meta,
    Kill,
}
