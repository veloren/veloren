pub mod meta;
use std::sync::{
    Arc,
    mpsc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::sleep;
use std::collections::HashMap;
use std::time::Duration;
use crate::{
    regionmanager::meta::{
        Server, Region, RegionId, RegionManagerMsg, RegionMIN, RegionMAX,
    },
    server::meta::{
        ServerMsg,
    },
    job::JobManager,
};

#[derive(Debug)]
pub struct RegionManager {
    tx: mpsc::Sender<RegionManagerMsg>,
    rx: mpsc::Receiver<ServerMsg>,
    running: Arc<AtomicBool>,
    servers: Vec<Server>,
    regions: HashMap<RegionId, Region>,
}

impl RegionManager{
    pub fn new(tx: mpsc::Sender<RegionManagerMsg>, rx: mpsc::Receiver<ServerMsg>) -> Self {

        let running = Arc::new(AtomicBool::new(true));

        let mut servers = vec!();
        let mut regions = HashMap::new();

        for x in RegionMIN..RegionMAX {
            for y in RegionMIN..RegionMAX {
                regions.insert((x,y), Region::new(None));
            }
        }

        Self {
            tx,
            rx,
            running,
            servers,
            regions,
        }
    }

    pub fn rearange(&mut self) {
        //This is a super intelligent algorithm which says which chunks should be handled by which server
        //It is widely important, that it causes as minimal shifting as necessary

        //.... fell f*** it for now
        for x in RegionMIN..RegionMAX {
            for y in RegionMIN..RegionMAX {
                if !self.servers.is_empty() {
                    let old = self.regions.get(&(x,y)).unwrap().server_id;

                    self.regions.get_mut(&(x,y)).unwrap().server_id = Some(((x as usize) % self.servers.len()) as u8);
                    if let Some(id) = old {
                        self.tx.send(RegionManagerMsg::TakeOverRegionFrom{region_id: (x,y), server_id: id as u64});
                    } else {
                        self.tx.send(RegionManagerMsg::CreateRegion{region_id: (x,y)});
                    }
                } else {
                    self.regions.get_mut(&(x,y)).unwrap().server_id = None;
                }
            }
        }
    }

    pub fn work(
        &mut self,
        //jm: &JobManager,
    ) -> bool {
        match self.rx.try_recv() {
            Ok(msg) => {
                match msg {
                    ServerMsg::Attach() => {
                        //ERROR i cannot acceess self here ...
                        self.servers.push(Server::new("Hello".to_string()) );
                        self.tx.send(RegionManagerMsg::Attached{server_id: self.servers.len() as u64 , seed: 1337});
                        error!("yay");
                        println!("attached");
                        self.rearange();
                    }
                }
            },
            Err(e) => {
                //panic!("Work error {:?}", e);
            }
        }

        sleep(Duration::from_millis(10));
        self.running.load(Ordering::Relaxed)
    }
}
