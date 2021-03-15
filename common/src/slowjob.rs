use std::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{Ordering, AtomicU64};
use core::any::Any;
use crossbeam_channel::{Receiver, Sender};

type LimitFn = dyn Fn(u64) -> u64;

/// a slow job is a CPU heavy task, that is not I/O blocking.
/// It usually takes longer than a tick to compute, so it's outsourced
/// Internally the rayon threadpool is used to calculate t
pub struct SlowJobGroup<D> {
    name: String,
    next_id: Arc<AtomicU64>,
    queue: Arc<RwLock<HashMap<u64, Queue>>>,
    local_running_jobs: Arc<AtomicU64>,
    global_running_jobs: Arc<AtomicU64>,
    receiver: Arc<Receiver<(String, D)>>,
    sender: Arc<Sender<(String, D)>>,
}

/// a slow job is a CPU heavy task, that is not I/O blocking.
/// It usually takes longer than a tick to compute, so it's outsourced
/// Internally the rayon threadpool is used to calculate t
pub struct SlowJobPool {
    next_id: AtomicU64,
    groups: RwLock<HashMap<String, Arc<SlowJobGroup<Box<dyn Any>>>>>,
    queue: RwLock<HashMap<String, HashMap<u64, Queue>>>,
    finished: RwLock<HashMap<String, Vec<Box<dyn Any>>>>,
    running_jobs: RwLock<HashMap<String, Arc<AtomicU64>>>,
    receiver: Receiver<(String, Box<dyn Any>)>,
    sender: Sender<(String, Box<dyn Any>)>,
    global_limit: Box<LimitFn>,
}

pub struct SlowJob {
    name: String,
    id: u64,
}

struct Queue {
    task: Box<dyn FnOnce() -> ()>,
    running_cnt: Arc<AtomicU64>,
}

impl<D> SlowJobGroup<D> where
    D: Any + Send + 'static
{
    /// spawn a new slow job
    pub fn spawn<F>(&self, name: &str, f: F) -> SlowJob where
        F: FnOnce() -> D + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let local_running_jobs_clone = Arc::clone(&self.local_running_jobs);
        let global_running_jobs_clone = Arc::clone(&self.global_running_jobs);
        let sender = self.sender.clone();
        let name_clone = name.to_string();
        let queue = Queue {
            task: Box::new(move || {
                let result = f();
                let _ = sender.send((name_clone, result));
                local_running_jobs_clone.fetch_sub(1, Ordering::Relaxed);
                global_running_jobs_clone.fetch_sub(1, Ordering::Relaxed);
            }),
            running_cnt: Arc::clone(&self.local_running_jobs),
        };
        self.queue.write().unwrap().insert(id, queue);
        SlowJob {
            name: name.to_string(),
            id,
        }
    }

    pub fn cancel(&self, job: SlowJob) {
        self.queue.write().unwrap().remove(&job.id);
    }

    /// collect all slow jobs finished
    pub fn collect(&self, name: &str) -> Vec<D> {
        let mut result = vec!();
        for (name, data) in self.receiver.try_iter() {
            result.push(data);
        }
        result
    }
}


impl SlowJobPool {
    pub fn new() -> Self {
        let (sender,receiver) = crossbeam_channel::unbounded();
        Self {
            next_id: AtomicU64::new(0),
            groups: RwLock::new(HashMap::new()),
            queue: RwLock::new(HashMap::new()),
            finished: RwLock::new(HashMap::new()),
            running_jobs: RwLock::new(HashMap::new()),
            receiver,
            sender,
            global_limit: Box::new(|n| n/2 + n/4),
        }
    }

    pub fn get<D>(&self, name: &str) -> Arc<SlowJobGroup<D>> where D: Sized + Send + 'static {
        let lock = self.groups.write().unwrap();
        if let Some(group) = lock.get(name) {
            if group.type_id() == Arc<SlowJobGroup<Box>>
        };
        panic!("Unconfigured group name!");
    }

    fn maintain(&self) {
        /*
        let mut lock = self.queue.write().unwrap();
        if let Some(map) = lock.get_mut(&job.name) {
            map.remove(&job.id);
        }
         */

        //let d = rayon::spawn(f);
    }
}