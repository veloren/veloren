pub mod meta;
use std::sync::{
    mpsc,
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;

use crate::{
    job::JobManager,
    regionmanager::meta::RegionManagerMsg,
    region::Region,
    server::meta::RegionId,
    server::meta::ServerMsg,
};

/*
one server per physical host
*/
#[derive(Debug)]
pub struct Server {
    tx: mpsc::Sender<ServerMsg>,
    rx: mpsc::Receiver<RegionManagerMsg>,
    running: Arc<AtomicBool>,
    id: Option<u64>,
    seed: Option<u64>,
    state: u64,
    jobmanager: Arc<JobManager>,
    region: HashMap<RegionId,Region>,
}

impl Server {
    pub fn new(tx: mpsc::Sender<ServerMsg>, rx: mpsc::Receiver<RegionManagerMsg>, jobmanager: Arc<JobManager>) -> Self {
        let running = Arc::new(AtomicBool::new(true));

        Self {
            tx,
            rx,
            running,
            id: None,
            seed: None,
            state: 0,
            jobmanager: jobmanager.clone(),
            region: HashMap::new(),
        }
    }

    pub fn work(
        &mut self,
        //jm: &JobManager,
    ) -> bool {
        match self.state {
            0 => {
                self.tx.send(ServerMsg::Attach());
                self.state += 1;
            },
            _ => (),
        }

        match self.rx.try_recv() {
            Ok(msg) => {
                match msg {
                    RegionManagerMsg::Attached{server_id, seed} => {
                        self.id = Some(server_id);
                        self.seed = Some(seed);
                    },
                    RegionManagerMsg::NewServerInMesh{server_id, server_connection_details} => {
                        println!("new server found");
                    },
                    RegionManagerMsg::CreateRegion{region_id} => {
                        /*
                        let mut r = Region::new(region_id, self.jobmanager.clone());
                        r.block.make_at_least(Vec3::new(0,0,0), Vec3::new(65535,65535,65535), 9);
                        self.region.insert(region_id, r);
                        */
                        println!("create region");
                    },
                    RegionManagerMsg::TakeOverRegionFrom{region_id, server_id} => {
                        println!("new server in mesh");
                    },
                    _ => (),
                }
            },
            Err(e) => {
                //panic!("Work error {:?}", e);
                sleep(Duration::from_millis(10));
            }
        }


        self.running.load(Ordering::Relaxed)
    }
}
