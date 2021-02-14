use crate::{Client, ClientType, ServerInfo};
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use futures_channel::oneshot;
use futures_executor::block_on;
use futures_timer::Delay;
use futures_util::{select, FutureExt};
use network::{Network, Participant, Promises};
use std::{sync::Arc, thread, time::Duration};
use tracing::{debug, error, trace, warn};

pub(crate) struct ServerInfoPacket {
    pub info: ServerInfo,
    pub time: f64,
}

pub(crate) type IncomingClient = Client;

pub(crate) struct ConnectionHandler {
    _network: Arc<Network>,
    thread_handle: Option<thread::JoinHandle<()>>,
    pub client_receiver: Receiver<IncomingClient>,
    pub info_requester_receiver: Receiver<Sender<ServerInfoPacket>>,
    stop_sender: Option<oneshot::Sender<()>>,
}

/// Instead of waiting the main loop we are handling connections, especially
/// their slow network .await part on a different thread. We need to communicate
/// to the Server main thread sometimes though to get the current server_info
/// and time
impl ConnectionHandler {
    pub fn new(network: Network) -> Self {
        let network = Arc::new(network);
        let network_clone = Arc::clone(&network);
        let (stop_sender, stop_receiver) = oneshot::channel();

        let (client_sender, client_receiver) = unbounded::<IncomingClient>();
        let (info_requester_sender, info_requester_receiver) =
            bounded::<Sender<ServerInfoPacket>>(1);

        let thread_handle = Some(thread::spawn(|| {
            block_on(Self::work(
                network_clone,
                client_sender,
                info_requester_sender,
                stop_receiver,
            ));
        }));

        Self {
            _network: network,
            thread_handle,
            client_receiver,
            info_requester_receiver,
            stop_sender: Some(stop_sender),
        }
    }

    async fn work(
        network: Arc<Network>,
        client_sender: Sender<IncomingClient>,
        info_requester_sender: Sender<Sender<ServerInfoPacket>>,
        stop_receiver: oneshot::Receiver<()>,
    ) {
        let mut stop_receiver = stop_receiver.fuse();
        loop {
            let participant = match select!(
                _ = stop_receiver => None,
                p = network.connected().fuse() => Some(p),
            ) {
                None => break,
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

            match select!(
                _ = stop_receiver => None,
                e = Self::init_participant(participant, client_sender, info_requester_sender).fuse() => Some(e),
            ) {
                None => break,
                Some(Ok(())) => (),
                Some(Err(e)) => warn!(?e, "drop new participant, because an error occurred"),
            }
        }
    }

    async fn init_participant(
        participant: Participant,
        client_sender: Sender<IncomingClient>,
        info_requester_sender: Sender<Sender<ServerInfoPacket>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("New Participant connected to the server");
        let (sender, receiver) = bounded(1);
        info_requester_sender.send(sender)?;

        let reliable = Promises::ORDERED | Promises::CONSISTENCY;
        let reliablec = reliable | Promises::COMPRESSED;

        let general_stream = participant.open(3, reliablec).await?;
        let ping_stream = participant.open(2, reliable).await?;
        let mut register_stream = participant.open(3, reliablec).await?;
        let character_screen_stream = participant.open(3, reliablec).await?;
        let in_game_stream = participant.open(3, reliablec).await?;

        let server_data = receiver.recv()?;

        register_stream.send(server_data.info)?;

        const TIMEOUT: Duration = Duration::from_secs(5);
        let client_type = match select!(
            _ = Delay::new(TIMEOUT).fuse() => None,
            t = register_stream.recv::<ClientType>().fuse() => Some(t),
        ) {
            None => {
                debug!("Timeout for incoming client elapsed, aborting connection");
                return Ok(());
            },
            Some(client_type) => client_type?,
        };

        let client = Client::new(
            client_type,
            participant,
            server_data.time,
            general_stream,
            ping_stream,
            register_stream,
            character_screen_stream,
            in_game_stream,
        );

        client_sender.send(client)?;
        Ok(())
    }
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        let _ = self.stop_sender.take().unwrap().send(());
        trace!("blocking till ConnectionHandler is closed");
        self.thread_handle
            .take()
            .unwrap()
            .join()
            .expect("There was an error in ConnectionHandler, clean shutdown impossible");
        trace!("gracefully closed ConnectionHandler!");
    }
}
