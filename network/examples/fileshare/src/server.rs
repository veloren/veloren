use crate::commands::{Command, FileInfo, LocalCommand, RemoteInfo};
use async_std::{
    fs,
    path::PathBuf,
    sync::{Mutex, RwLock},
};
use futures::{channel::mpsc, future::FutureExt, stream::StreamExt};
use network::{Address, Network, Participant, Pid, Stream, PROMISES_CONSISTENCY, PROMISES_ORDERED};
use std::{collections::HashMap, sync::Arc};
use tracing::*;
use uvth::ThreadPoolBuilder;

#[derive(Debug)]
struct ControlChannels {
    command_receiver: mpsc::UnboundedReceiver<LocalCommand>,
}

pub struct Server {
    run_channels: Option<ControlChannels>,
    network: Network,
    served: RwLock<Vec<FileInfo>>,
    remotes: RwLock<HashMap<Pid, Arc<Mutex<RemoteInfo>>>>,
    receiving_files: Mutex<HashMap<u32, Option<String>>>,
}

impl Server {
    pub fn new() -> (Self, mpsc::UnboundedSender<LocalCommand>) {
        let (command_sender, command_receiver) = mpsc::unbounded();

        let thread_pool = ThreadPoolBuilder::new().build();
        let network = Network::new(Pid::new(), &thread_pool);

        let run_channels = Some(ControlChannels { command_receiver });
        (
            Server {
                run_channels,
                network,
                served: RwLock::new(vec![]),
                remotes: RwLock::new(HashMap::new()),
                receiving_files: Mutex::new(HashMap::new()),
            },
            command_sender,
        )
    }

    pub async fn run(mut self, address: Address) {
        let run_channels = self.run_channels.take().unwrap();

        self.network.listen(address).await.unwrap();

        futures::join!(
            self.command_manager(run_channels.command_receiver,),
            self.connect_manager(),
        );
    }

    async fn command_manager(&self, command_receiver: mpsc::UnboundedReceiver<LocalCommand>) {
        trace!("start command_manager");
        command_receiver
            .for_each_concurrent(None, async move |cmd| {
                match cmd {
                    LocalCommand::Shutdown => {
                        println!("shutting down service");
                        return;
                    },
                    LocalCommand::Disconnect => {
                        self.remotes.write().await.clear();
                        for (_, p) in self.network.participants().await.drain() {
                            self.network.disconnect(p).await.unwrap();
                        }
                        println!("disconnecting all connections");
                        return;
                    },
                    LocalCommand::Connect(addr) => {
                        println!("trying to connect to: {:?}", &addr);
                        match self.network.connect(addr.clone()).await {
                            Ok(p) => self.loop_participant(p).await,
                            Err(e) => {
                                println!("failled to connect to {:?}, err: {:?}", &addr, e);
                            },
                        }
                    },
                    LocalCommand::Serve(fileinfo) => {
                        self.served.write().await.push(fileinfo.clone());
                        println!("serving file: {:?}", fileinfo.path);
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
                            let mut ri = ri.lock().await;
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
        trace!("stop command_manager");
    }

    async fn connect_manager(&self) {
        trace!("start connect_manager");
        let iter = futures::stream::unfold((), |_| {
            self.network.connected().map(|r| r.ok().map(|v| (v, ())))
        });

        iter.for_each_concurrent(/* limit */ None, async move |participant| {
            self.loop_participant(participant).await;
        })
        .await;
        trace!("stop connect_manager");
    }

    async fn loop_participant(&self, p: Arc<Participant>) {
        if let (Ok(cmd_out), Ok(file_out), Ok(cmd_in), Ok(file_in)) = (
            p.open(15, PROMISES_CONSISTENCY | PROMISES_ORDERED).await,
            p.open(40, PROMISES_CONSISTENCY).await,
            p.opened().await,
            p.opened().await,
        ) {
            debug!(?p, "connection successfully initiated");
            let id = p.remote_pid();
            let ri = Arc::new(Mutex::new(RemoteInfo::new(cmd_out, file_out, p)));
            self.remotes.write().await.insert(id, ri.clone());
            futures::join!(
                self.handle_remote_cmd(cmd_in, ri.clone()),
                self.handle_files(file_in, ri.clone()),
            );
        }
    }

    async fn handle_remote_cmd(&self, mut stream: Stream, remote_info: Arc<Mutex<RemoteInfo>>) {
        while let Ok(msg) = stream.recv::<Command>().await {
            println!("got message: {:?}", &msg);
            match msg {
                Command::List => {
                    info!("request to send my list");
                    let served = self.served.read().await.clone();
                    stream.send(served).unwrap();
                },
                Command::Get(id) => {
                    for file_info in self.served.read().await.iter() {
                        if file_info.id() == id {
                            info!("request to send file i got, sending it");
                            if let Ok(data) = file_info.load().await {
                                match remote_info.lock().await.file_out.send((file_info, data)) {
                                    Ok(_) => debug!("send file"),
                                    Err(e) => error!(?e, "sending file failed"),
                                }
                            } else {
                                warn!("cannot send file as loading failed, oes it still exist?");
                            }
                        }
                    }
                },
            }
        }
    }

    async fn handle_files(&self, mut stream: Stream, _remote_info: Arc<Mutex<RemoteInfo>>) {
        while let Ok((fi, data)) = stream.recv::<(FileInfo, Vec<u8>)>().await {
            debug!(?fi, "got file");
            let path = self.receiving_files.lock().await.remove(&fi.id()).flatten();
            let path: PathBuf = match &path {
                Some(path) => shellexpand::tilde(&path).parse().unwrap(),
                None => {
                    let mut path = std::env::current_dir().unwrap();
                    path.push(fi.path().file_name().unwrap());
                    trace!("no path provided, saving down to {:?}", path);
                    PathBuf::from(path)
                },
            };
            debug!("received file, going to save it under {:?}", path);
            fs::write(path, data).await.unwrap();
        }
    }
}

fn print_fileinfos(infos: &Vec<FileInfo>) {
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
