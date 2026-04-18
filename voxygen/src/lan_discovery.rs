/// Passive LAN server discovery via UDP broadcast.
///
/// When a Nova-Forge LAN co-op server starts it periodically broadcasts a small
/// UDP datagram on [`DISCOVERY_PORT`].  Any client on the same subnet that has
/// a [`LanDiscovery`] running will receive those datagrams and surface the
/// server in the server browser automatically — no manual IP entry required.
///
/// # Packet format (v3)
/// ```text
/// NOVA_FORGE_LAN\0  (15 bytes, 14-char magic + NUL)
/// port_lo  port_hi  (2 bytes little-endian u16)
/// player_count      (1 byte u8)
/// player_cap        (1 byte u8, 0 = unknown/unlimited)
/// version_len       (1 byte u8, length of the UTF-8 version string)
/// version_bytes     (version_len bytes, e.g. "0.18.0-dev")
/// <UTF-8 server name, up to 64 bytes>
/// ```
/// Total on-wire size is always ≤ 100 bytes, well inside a single UDP frame.
use std::{
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tracing::{debug, trace, warn};

/// UDP port used for both broadcasting (server) and listening (client).
pub const DISCOVERY_PORT: u16 = 14005;

/// Magic header prefix that identifies a Nova-Forge LAN discovery packet.
const MAGIC: &[u8] = b"NOVA_FORGE_LAN\0";
/// How long between server broadcasts.
const BROADCAST_INTERVAL: Duration = Duration::from_secs(5);
/// How long a discovered entry remains valid after the last broadcast.
const ENTRY_TTL: Duration = Duration::from_secs(15);
/// Read timeout on the listener socket so the thread can check the stop flag.
const LISTEN_TIMEOUT: Duration = Duration::from_secs(1);

// ────────────────────────────────────────────────────────────────────────────
// Public types
// ────────────────────────────────────────────────────────────────────────────

/// A LAN server that has been seen recently.
#[derive(Debug, Clone)]
pub struct DiscoveredServer {
    /// The address guests should use to connect (source IP + game port from
    /// the packet, not the discovery port).
    pub address: String,
    /// Server name announced in the broadcast.
    pub name: String,
    /// Number of players currently connected (as of the last broadcast).
    pub player_count: u8,
    /// Maximum player capacity (0 = unknown / unlimited).
    pub player_cap: u8,
    /// Game version string of the server (e.g. `"0.18.0-dev"`), or empty if
    /// the server sent an older packet format that lacked this field.
    pub version: String,
    /// When the most recent broadcast from this server was received.
    pub last_seen: Instant,
}

/// Manages the background listener thread and the shared list of discovered
/// LAN servers.
///
/// Dropped automatically when the client exits the main-menu state;
/// the background thread exits within one [`LISTEN_TIMEOUT`] tick.
pub struct LanDiscovery {
    servers: Arc<Mutex<Vec<DiscoveredServer>>>,
    stop: Arc<AtomicBool>,
    _thread: Option<thread::JoinHandle<()>>,
}

impl LanDiscovery {
    /// Spawn the listener thread and return a handle.
    ///
    /// Failure to bind the socket is treated as non-fatal: the struct is still
    /// returned but no entries will ever be discovered.
    pub fn start() -> Self {
        let servers: Arc<Mutex<Vec<DiscoveredServer>>> = Default::default();
        let stop = Arc::new(AtomicBool::new(false));

        let servers_clone = Arc::clone(&servers);
        let stop_clone = Arc::clone(&stop);

        let handle = match UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT))
        {
            Ok(socket) => {
                socket
                    .set_read_timeout(Some(LISTEN_TIMEOUT))
                    .unwrap_or_else(|e| warn!(?e, "Failed to set LAN discovery socket timeout"));

                let builder = thread::Builder::new().name("lan-discovery-listener".into());
                let handle = builder
                    .spawn(move || {
                        listener_thread(socket, servers_clone, stop_clone);
                    })
                    .ok();
                handle
            },
            Err(e) => {
                debug!(?e, "Could not bind LAN discovery socket (non-fatal)");
                None
            },
        };

        Self {
            servers,
            stop,
            _thread: handle,
        }
    }

    /// Returns a snapshot of currently live LAN servers, evicting entries
    /// whose last broadcast is older than [`ENTRY_TTL`].
    pub fn snapshot(&self) -> Vec<DiscoveredServer> {
        let mut guard = self.servers.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        guard.retain(|s| now.duration_since(s.last_seen) < ENTRY_TTL);
        guard.clone()
    }
}

impl Drop for LanDiscovery {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // The thread will exit on its next 1-second read-timeout tick.
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────────────

fn listener_thread(
    socket: UdpSocket,
    servers: Arc<Mutex<Vec<DiscoveredServer>>>,
    stop: Arc<AtomicBool>,
) {
    let mut buf = [0u8; 256];
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match socket.recv_from(&mut buf) {
            Ok((len, src)) => {
                if let Some(entry) = parse_packet(&buf[..len]) {
                    let address = format!("{}:{}", src.ip(), entry.port);
                    trace!(%address, name = %entry.name, "LAN server discovered");
                    let mut guard = servers.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(existing) = guard.iter_mut().find(|s| s.address == address) {
                        existing.name = entry.name;
                        existing.player_count = entry.player_count;
                        existing.player_cap = entry.player_cap;
                        existing.version = entry.version;
                        existing.last_seen = Instant::now();
                    } else {
                        guard.push(DiscoveredServer {
                            address,
                            name: entry.name,
                            player_count: entry.player_count,
                            player_cap: entry.player_cap,
                            version: entry.version,
                            last_seen: Instant::now(),
                        });
                    }
                }
            },
            // WouldBlock / TimedOut are expected — just loop again.
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue
            },
            Err(e) => {
                debug!(?e, "LAN discovery recv error");
            },
        }
    }
}

struct ParsedPacket {
    port: u16,
    player_count: u8,
    player_cap: u8,
    version: String,
    name: String,
}

fn parse_packet(data: &[u8]) -> Option<ParsedPacket> {
    let data = data.strip_prefix(MAGIC)?;
    // Minimum: 2 (port) + 2 (player_count + player_cap) + 1 (version_len) = 5 bytes after magic.
    if data.len() < 5 {
        return None;
    }
    let port = u16::from_le_bytes([data[0], data[1]]);
    if port == 0 {
        return None;
    }
    let player_count = data[2];
    let player_cap = data[3];
    let version_len = data[4] as usize;
    // Ensure version_len doesn't run past the buffer.
    if data.len() < 5 + version_len {
        return None;
    }
    let version = std::str::from_utf8(&data[5..5 + version_len])
        .ok()?
        .to_owned();
    // Cap the name field at 64 bytes to match encode_packet's upper bound, even
    // if a malformed packet tries to send more.
    let name_start = 5 + version_len;
    let end = data.len().min(name_start + 64);
    let name = std::str::from_utf8(&data[name_start..end])
        .ok()?
        .trim()
        .to_owned();
    Some(ParsedPacket {
        port,
        player_count,
        player_cap,
        version,
        name,
    })
}

// ────────────────────────────────────────────────────────────────────────────
// Broadcaster  (used by the LAN co-op server thread)
// ────────────────────────────────────────────────────────────────────────────

/// Encode a discovery packet.
///
/// `player_count` is the number of connected players; `player_cap` is the
/// server's maximum (0 = unknown / unlimited).  `version` is the game version
/// string (e.g. `"0.18.0-dev"`), truncated to 16 bytes if longer.
pub fn encode_packet(
    port: u16,
    player_count: u8,
    player_cap: u8,
    version: &str,
    server_name: &str,
) -> Vec<u8> {
    let version_bytes = version.as_bytes();
    // Truncate version to at most 16 bytes.
    let version_bytes = &version_bytes[..version_bytes.len().min(16)];
    let name_bytes = server_name.as_bytes();
    // Truncate the name to 64 bytes to keep the packet small.
    let name_bytes = &name_bytes[..name_bytes.len().min(64)];
    let mut pkt =
        Vec::with_capacity(MAGIC.len() + 4 + 1 + version_bytes.len() + name_bytes.len());
    pkt.extend_from_slice(MAGIC);
    pkt.extend_from_slice(&port.to_le_bytes());
    pkt.push(player_count);
    pkt.push(player_cap);
    pkt.push(version_bytes.len() as u8);
    pkt.extend_from_slice(version_bytes);
    pkt.extend_from_slice(name_bytes);
    pkt
}

/// Spawn a background thread that broadcasts LAN server presence until
/// `stop` is set.
///
/// `player_count` is an `Arc<AtomicU8>` updated by the server tick loop so
/// the broadcaster can include a live player count in each packet without
/// locking the server.  `version` is the game version string included in
/// each packet so the client browser can show which build is running.
///
/// This is called from the LAN co-op server thread immediately after the
/// server has finished initialising, so the address-in-use failure mode
/// should not occur in practice (we bind to an ephemeral source port).
pub fn start_broadcaster(
    port: u16,
    server_name: String,
    version: String,
    player_cap: u8,
    player_count: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
) {
    let builder = thread::Builder::new().name("lan-discovery-broadcaster".into());
    let _ = builder.spawn(move || {
        let socket = match UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)) {
            Ok(s) => s,
            Err(e) => {
                warn!(?e, "Failed to create LAN discovery broadcast socket");
                return;
            },
        };
        if let Err(e) = socket.set_broadcast(true) {
            warn!(?e, "Failed to enable UDP broadcast (LAN discovery disabled)");
            return;
        }
        let dest: std::net::SocketAddr =
            SocketAddrV4::new(Ipv4Addr::BROADCAST, DISCOVERY_PORT).into();
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            // Rebuild the packet each broadcast so the player count is live.
            let count = player_count.load(Ordering::Relaxed);
            let packet = encode_packet(port, count, player_cap, &version, &server_name);
            if let Err(e) = socket.send_to(&packet, dest) {
                debug!(?e, "LAN discovery broadcast send failed");
            }
            // Sleep in small increments so the stop flag is checked promptly.
            let mut remaining = BROADCAST_INTERVAL;
            while remaining > Duration::ZERO && !stop.load(Ordering::Relaxed) {
                let step = remaining.min(Duration::from_millis(200));
                thread::sleep(step);
                remaining = remaining.saturating_sub(step);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let pkt = encode_packet(14004, 2, 8, "0.18.0-dev", "My LAN Server");
        let parsed = parse_packet(&pkt).expect("parse failed");
        assert_eq!(parsed.port, 14004);
        assert_eq!(parsed.player_count, 2);
        assert_eq!(parsed.player_cap, 8);
        assert_eq!(parsed.version, "0.18.0-dev");
        assert_eq!(parsed.name, "My LAN Server");
    }

    #[test]
    fn name_truncation() {
        let long_name = "A".repeat(100);
        let pkt = encode_packet(14004, 0, 0, "1.0", &long_name);
        let parsed = parse_packet(&pkt).expect("parse failed");
        assert_eq!(parsed.name.len(), 64);
    }

    #[test]
    fn version_truncation() {
        let long_version = "v".repeat(100);
        let pkt = encode_packet(14004, 0, 0, &long_version, "Server");
        let parsed = parse_packet(&pkt).expect("parse failed");
        assert_eq!(parsed.version.len(), 16);
        assert_eq!(parsed.name, "Server");
    }

    #[test]
    fn empty_version() {
        let pkt = encode_packet(14004, 1, 4, "", "No Version Server");
        let parsed = parse_packet(&pkt).expect("parse failed");
        assert_eq!(parsed.version, "");
        assert_eq!(parsed.name, "No Version Server");
    }

    #[test]
    fn rejects_invalid_magic() {
        let bad = b"GARBAGE\0\x94\x36hello";
        assert!(parse_packet(bad).is_none());
    }

    #[test]
    fn rejects_zero_port() {
        let mut pkt = encode_packet(0, 1, 8, "1.0", "zero port");
        // Ensure the port bytes are both zero after magic.
        let ml = MAGIC.len();
        pkt[ml] = 0;
        pkt[ml + 1] = 0;
        assert!(parse_packet(&pkt).is_none());
    }
}
