use crate::commands::{Command, FileInfo, LocalCommand, RemoteInfo};
use futures_util::StreamExt;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{
    fs, join,
    runtime::Runtime,
    sync::{mpsc, Mutex, RwLock},
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::*;
use veloren_network::{ListenAddr, Network, Participant, Pid, Promises, Stream};

#[derive(Debug)]
struct ControlChannels {
    command_receiver: mpsc::UnboundedReceiver<LocalCommand>,
}

struct Shared {
    served: RwLock<Vec<FileInfo>>,
    remotes: RwLock<HashMap<Pid, Arc<Mutex<RemoteInfo>>>>,
    receiving_files: Mutex<HashMap<u32, Option<String>>>,
}

pub struct Server {
    run_channels: ControlChannels,
    server: Network,
    client: Network,
    shared: Shared,
}

impl Server {
    pub fn new(runtime: Arc<Runtime>) -> (Self, mpsc::UnboundedSender<LocalCommand>) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let server = Network::new(Pid::new(), &runtime);
        let client = Network::new(Pid::new(), &runtime);

        let run_channels = ControlChannels { command_receiver };
        (
            Server {
                run_channels,
                server,
                client,
                shared: Shared {
                    served: RwLock::new(vec![]),
                    remotes: RwLock::new(HashMap::new()),
                    receiving_files: Mutex::new(HashMap::new()),
                },
            },
            command_sender,
        )
    }

    pub async fn run(self, address: ListenAddr) {
        let run_channels = self.run_channels;

        self.server.listen(address).await.unwrap();

        join!(
            self.shared
                .command_manager(self.client, run_channels.command_receiver),
            self.shared.connect_manager(self.server),
        );
    }
}

impl Shared {
    async fn command_manager(
        &self,
        client: Network,
        command_receiver: mpsc::UnboundedReceiver<LocalCommand>,
    ) {
        trace!("Start command_manager");
        let command_receiver = UnboundedReceiverStream::new(command_receiver);
        command_receiver
            .for_each_concurrent(None, |cmd| async {
                match cmd {
                    LocalCommand::Shutdown => println!("Shutting down service"),
                    LocalCommand::Disconnect => {
                        self.remotes.write().await.clear();
                        println!("Disconnecting all connections");
                    },
                    LocalCommand::Connect(addr) => {
                        println!("Trying to connect to: {:?}", &addr);
                        match client.connect(addr.clone()).await {
                            Ok(p) => self.loop_participant(p).await,
                            Err(e) => println!("Failed to connect to {:?}, err: {:?}", &addr, e),
                        }
                    },
                    LocalCommand::Serve(fileinfo) => {
                        self.served.write().await.push(fileinfo.clone());
                        println!("Serving file: {:?}", fileinfo.path);
                    },
                    LocalCommand::List => {
                        let mut total_file_infos = vec![];
                        for ri in self.remotes.read().await.values() {
                            let mut ri = ri.lock().await;
                            ri.cmd_out.send(Command::List).unwrap();
                            let mut file_infos = ri.cmd_out.recv::<Vec<FileInfo>>().await.unwrap();
                            ri.insert_infos(file_infos.clone());
                            total_file_infos.append(&mut file_infos);
                        }
                        print_fileinfos(&total_file_infos);
                    },
                    LocalCommand::Get(id, path) => {
                        // i dont know the owner, just broadcast, i am laaaazyyy
                        for ri in self.remotes.read().await.values() {
                            let ri = ri.lock().await;
                            if ri.get_info(id).is_some() {
                                //found provider, send request.
                                self.receiving_files.lock().await.insert(id, path.clone());
                                ri.cmd_out.send(Command::Get(id)).unwrap();
                                // the answer is handled via the other stream!
                                break;
                            }
                        }
                    },
                }
            })
            .await;
        trace!("Stop command_manager");
    }

    async fn connect_manager(&self, network: Network) {
        trace!("Start connect_manager");
        let iter = futures_util::stream::unfold(network, async move |mut network| {
            network.connected().await.ok().map(|v| (v, network))
        });

        iter.for_each_concurrent(/* limit */ None, |participant| async {
            self.loop_participant(participant).await;
        })
        .await;
        trace!("Stop connect_manager");
    }

    async fn loop_participant(&self, mut p: Participant) {
        if let (Ok(cmd_out), Ok(file_out), Ok(cmd_in), Ok(file_in)) = (
            p.open(3, Promises::ORDERED | Promises::CONSISTENCY, 0)
                .await,
            p.open(6, Promises::CONSISTENCY, 0).await,
            p.opened().await,
            p.opened().await,
        ) {
            debug!(?p, "Connection successfully initiated");
            let id = p.remote_pid();
            let ri = Arc::new(Mutex::new(RemoteInfo::new(cmd_out, file_out, p)));
            self.remotes.write().await.insert(id, ri.clone());
            join!(
                self.handle_remote_cmd(cmd_in, ri.clone()),
                self.handle_files(file_in, ri.clone()),
            );
        }
    }

    async fn handle_remote_cmd(&self, mut stream: Stream, remote_info: Arc<Mutex<RemoteInfo>>) {
        while let Ok(msg) = stream.recv::<Command>().await {
            println!("Got message: {:?}", &msg);
            match msg {
                Command::List => {
                    info!("Request to send my list");
                    let served = self.served.read().await.clone();
                    stream.send(served).unwrap();
                },
                Command::Get(id) => {
                    for file_info in self.served.read().await.iter() {
                        if file_info.id() == id {
                            info!("Request to send file i got, sending it");
                            if let Ok(data) = file_info.load().await {
                                match remote_info.lock().await.file_out.send((file_info, data)) {
                                    Ok(_) => debug!("send file"),
                                    Err(e) => error!(?e, "sending file failed"),
                                }
                            } else {
                                warn!("Cannot send file as loading failed, oes it still exist?");
                            }
                        }
                    }
                },
            }
        }
    }

    async fn handle_files(&self, mut stream: Stream, _remote_info: Arc<Mutex<RemoteInfo>>) {
        while let Ok((fi, data)) = stream.recv::<(FileInfo, Vec<u8>)>().await {
            debug!(?fi, "Got file");
            let path = self.receiving_files.lock().await.remove(&fi.id()).flatten();
            let path: PathBuf = match &path {
                Some(path) => shellexpand::tilde(&path).parse().unwrap(),
                None => {
                    let mut path = std::env::current_dir().unwrap();
                    path.push(fi.path().file_name().unwrap());
                    trace!("No path provided, saving down to {:?}", path);
                    path
                },
            };
            debug!("Received file, going to save it under {:?}", path);
            fs::write(path, data).await.unwrap();
        }
    }
}

fn print_fileinfos(infos: &[FileInfo]) {
    let mut i = 0;
    for info in infos {
        let bytes = info.size;
        match bytes {
            0..100_000 => println!("{}: {}bytes '{}'", info.id(), bytes, info.path),
            100_000..100_000_000 => {
                println!("{}: {}bytes '{}'", info.id(), bytes / 1024, info.path)
            },
            _ => println!(
                "{}: {}bytes '{}'",
                info.id(),
                bytes / 1024 / 1024,
                info.path
            ),
        }
        i += 1;
    }
    println!("-- {} files available", i);
}
