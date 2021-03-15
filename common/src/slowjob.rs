use std::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{Ordering, AtomicU64};
use core::any::Any;
use crossbeam_channel::{Receiver, Sender};

/// a slow job is a CPU heavy task, that is not I/O blocking.
/// It usually takes longer than a tick to compute, so it's outsourced
/// Internally the rayon threadpool is used to calculate t
pub struct SlowJobPool {
    next_id: AtomicU64,
    queue: RwLock<HashMap<String, HashMap<u64, Queue>>>,
    finished: RwLock<HashMap<String, Vec<Box<dyn Any>>>>,
    running_jobs: RwLock<HashMap<String, Arc<AtomicU64>>>,
    receiver: Receiver<(String, Box<dyn Any>)>,
    sender: Sender<(String, Box<dyn Any>)>,
}

pub struct SlowJob {
    name: String,
    id: u64,
}

struct Queue {
    task: Box<dyn FnOnce() -> ()>,
    running_cnt: Arc<AtomicU64>,
}


impl SlowJobPool {
    pub fn new() -> Self {
        let (sender,receiver) = crossbeam_channel::unbounded();
        Self {
            next_id: AtomicU64::new(0),
            queue: RwLock::new(HashMap::new()),
            finished: RwLock::new(HashMap::new()),
            receiver,
            sender
        }
    }

    /// spawn a new slow job
    pub fn spawn<F, D>(&self, name: &str, f: F) -> SlowJob where
        F: FnOnce() -> D + Send + 'static,
        D: Any + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let running_cnt = {
            let mut lock = self.running_jobs.write().unwrap();
            lock.entry(name.to_string()).or_default().clone()
        };
        let running_cnt_clone = Arc::clone(&running_cnt);
        let sender = self.sender.clone();
        let name_clone = name.to_string();
        let queue = Queue {
            task: Box::new(move || {
                let result = f();
                let _ = sender.send((name_clone, Box::new(result)));
                running_cnt_clone.fetch_sub(1, Ordering::Relaxed);
            }),
            running_cnt,
        };
        {
            let mut lock = self.queue.write().unwrap();
            lock.entry(name.to_string()).or_default().insert(id, queue);
        }
        SlowJob {
            name: name.to_string(),
            id,
        }
    }

    pub fn cancel(&self, job: SlowJob) {
        let mut lock = self.queue.write().unwrap();
        if let Some(map) = lock.get_mut(&job.name) {
            map.remove(&job.id);
        }
    }

    /// collect all slow jobs finished
    pub fn collect(&self, name: &str) -> Vec<Box<dyn Any>> {
        let mut lock = self.finished.write().unwrap();
        for (name, data) in self.receiver.try_iter() {
            lock.entry(name).or_default().push(data);
        }
        lock.remove(name).unwrap_or_default()
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