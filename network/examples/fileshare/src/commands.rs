use async_std::{
    fs,
    path::{Path, PathBuf},
};
use network::{ProtocolAddr, Participant, Stream};
use rand::Rng;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Debug)]
pub enum LocalCommand {
    Shutdown,
    Disconnect,
    Connect(ProtocolAddr),
    List,
    Serve(FileInfo),
    Get(u32, Option<String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    List,
    Get(u32),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FileInfo {
    id: u32,
    pub path: String,
    pub size: u64,
    pub hash: String,
}

pub struct RemoteInfo {
    infos: HashMap<u32, FileInfo>,
    _participant: Participant,
    pub cmd_out: Stream,
    pub file_out: Stream,
}

impl FileInfo {
    pub async fn new(path: &Path) -> Option<Self> {
        let mt = match fs::metadata(&path).await {
            Err(e) => {
                println!(
                    "Cannot get metadata for file: {:?}, does it exist? Error: {:?}",
                    &path, &e
                );
                return None;
            },
            Ok(mt) => mt,
        };
        let size = mt.len();
        Some(Self {
            id: rand::thread_rng().gen(),
            path: path.as_os_str().to_os_string().into_string().unwrap(),
            size,
            hash: "<none>".to_owned(),
        })
    }

    pub async fn load(&self) -> Result<Vec<u8>, std::io::Error> { fs::read(self.path()).await }

    pub fn id(&self) -> u32 { self.id }

    pub fn path(&self) -> PathBuf { self.path.parse().unwrap() }
}

impl RemoteInfo {
    pub fn new(cmd_out: Stream, file_out: Stream, participant: Participant) -> Self {
        Self {
            infos: HashMap::new(),
            _participant: participant,
            cmd_out,
            file_out,
        }
    }

    pub fn get_info(&self, id: u32) -> Option<FileInfo> { self.infos.get(&id).map(|fi| fi.clone()) }

    pub fn insert_infos(&mut self, mut fi: Vec<FileInfo>) {
        for fi in fi.drain(..) {
            self.infos.insert(fi.id(), fi);
        }
    }
}
