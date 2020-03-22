use async_std::{sync::RwLock, task};
use futures::{
    channel::{mpsc, oneshot},
    executor::ThreadPool,
    sink::SinkExt,
};
use std::sync::{atomic::AtomicU64, Arc};
use veloren_network::{Network, Pid, Scheduler};
mod helper;
use std::collections::HashMap;
use tracing::*;
use uvth::ThreadPoolBuilder;

#[test]
fn network() {
    let (_, _) = helper::setup(true, 100);
    {
        let addr1 = helper::tcp();
        let pool = ThreadPoolBuilder::new().num_threads(2).build();
        let n1 = Network::new(Pid::fake(1), &pool);
        let n2 = Network::new(Pid::fake(2), &pool);

        n1.listen(addr1.clone()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let pid1 = task::block_on(n2.connect(addr1)).unwrap();
        warn!("yay connected");

        let pid2 = task::block_on(n1.connected()).unwrap();
        warn!("yay connected");

        let mut sid1_p1 = task::block_on(pid1.open(10, 0)).unwrap();
        let mut sid1_p2 = task::block_on(pid2.opened()).unwrap();

        task::block_on(sid1_p1.send("Hello World")).unwrap();
        let m1: Result<String, _> = task::block_on(sid1_p2.recv());
        assert_eq!(m1, Ok("Hello World".to_string()));

        //assert_eq!(pid, Pid::fake(1));

        std::thread::sleep(std::time::Duration::from_secs(10));
    }
    std::thread::sleep(std::time::Duration::from_secs(2));
}

#[test]
#[ignore]
fn scheduler() {
    let (_, _) = helper::setup(true, 100);
    let addr = helper::tcp();
    let (scheduler, mut listen_tx, _, _, _) = Scheduler::new(Pid::new());
    task::block_on(listen_tx.send(addr)).unwrap();
    task::block_on(scheduler.run());
}

#[test]
#[ignore]
fn channel_creator_test() {
    let (_, _) = helper::setup(true, 100);
    let (_end_sender, end_receiver) = oneshot::channel::<()>();
    let (part_out_sender, _part_out_receiver) = mpsc::unbounded();
    let (configured_sender, _configured_receiver) = mpsc::unbounded::<(u64, Pid, u64)>();
    let addr = helper::tcp();
    task::block_on(async {
        Scheduler::channel_creator(
            Arc::new(AtomicU64::new(0)),
            Pid::new(),
            addr,
            end_receiver,
            Arc::new(ThreadPool::new().unwrap()),
            part_out_sender,
            configured_sender,
            Arc::new(RwLock::new(HashMap::new())),
        )
        .await;
    });
}
