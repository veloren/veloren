use crate::{Client, ClientType, ServerInfo};
use crossbeam::{bounded, unbounded, Receiver, Sender};
use futures_executor::block_on;
use futures_timer::Delay;
use futures_util::{select, FutureExt};
use network::{Network, Participant, Promises};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tracing::{debug, error, trace, warn};

pub(crate) struct ServerInfoPacket {
    pub info: ServerInfo,
    pub time: f64,
}

pub(crate) type ConnectionDataPacket = Client;

pub(crate) struct ConnectionHandler {
    _network: Arc<Network>,
    thread_handle: Option<thread::JoinHandle<()>>,
    pub client_receiver: Receiver<ConnectionDataPacket>,
    pub info_requester_receiver: Receiver<Sender<ServerInfoPacket>>,
    running: Arc<AtomicBool>,
}

/// Instead of waiting the main loop we are handling connections, especially
/// their slow network .await part on a different thread. We need to communicate
/// to the Server main thread sometimes tough to get the current server_info and
/// time
impl ConnectionHandler {
    pub fn new(network: Network) -> Self {
        let network = Arc::new(network);
        let network_clone = Arc::clone(&network);
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let (client_sender, client_receiver) = unbounded::<ConnectionDataPacket>();
        let (info_requester_sender, info_requester_receiver) =
            bounded::<Sender<ServerInfoPacket>>(1);

        let thread_handle = Some(thread::spawn(|| {
            block_on(Self::work(
                network_clone,
                client_sender,
                info_requester_sender,
                running_clone,
            ));
        }));

        Self {
            _network: network,
            thread_handle,
            client_receiver,
            info_requester_receiver,
            running,
        }
    }

    async fn work(
        network: Arc<Network>,
        client_sender: Sender<ConnectionDataPacket>,
        info_requester_sender: Sender<Sender<ServerInfoPacket>>,
        running: Arc<AtomicBool>,
    ) {
        while running.load(Ordering::Relaxed) {
            const TIMEOUT: Duration = Duration::from_secs(5);
            let participant = match select!(
                _ = Delay::new(TIMEOUT).fuse() => None,
                p = network.connected().fuse() => Some(p),
            ) {
                None => continue, //check condition
                Some(Ok(p)) => p,
                Some(Err(e)) => {
                    error!(
                        ?e,
                        "Stopping Conection Handler, no new connections can be made to server now!"
                    );
                    break;
                },
            };

            let client_sender = client_sender.clone();
            let info_requester_sender = info_requester_sender.clone();

            match Self::init_participant(participant, client_sender, info_requester_sender).await {
                Ok(_) => (),
                Err(e) => warn!(?e, "drop new participant, because an error occurred"),
            }
        }
    }

    async fn init_participant(
        participant: Participant,
        client_sender: Sender<ConnectionDataPacket>,
        info_requester_sender: Sender<Sender<ServerInfoPacket>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("New Participant connected to the server");
        let (sender, receiver) = bounded(1);
        info_requester_sender.send(sender)?;

        let reliable = Promises::ORDERED | Promises::CONSISTENCY;
        let reliablec = reliable | Promises::COMPRESSED;

        let general_stream = participant.open(10, reliablec).await?;
        let ping_stream = participant.open(5, reliable).await?;
        let mut register_stream = participant.open(10, reliablec).await?;
        let character_screen_stream = participant.open(10, reliablec).await?;
        let in_game_stream = participant.open(10, reliablec).await?;

        let server_data = receiver.recv()?;

        register_stream.send(server_data.info)?;

        const TIMEOUT: Duration = Duration::from_secs(5);
        let client_type = match select!(
            _ = Delay::new(TIMEOUT).fuse() => None,
            t = register_stream.recv::<ClientType>().fuse() => Some(t),
        ) {
            None => {
                debug!("slow client connection detected, dropping it");
                return Ok(());
            },
            Some(client_type) => client_type?,
        };

        let client = Client {
            registered: false,
            client_type,
            in_game: None,
            participant: std::sync::Mutex::new(Some(participant)),
            singleton_stream: general_stream,
            ping_stream,
            register_stream,
            in_game_stream,
            character_screen_stream,
            network_error: std::sync::atomic::AtomicBool::new(false),
            last_ping: server_data.time,
            login_msg_sent: false,
        };

        client_sender.send(client)?;
        Ok(())
    }
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        trace!("blocking till ConnectionHandler is closed");
        self.thread_handle
            .take()
            .unwrap()
            .join()
            .expect("There was an error in ConnectionHandler, clean shutdown impossible");
        trace!("gracefully closed ConnectionHandler!");
    }
}
