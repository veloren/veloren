use common::slowjob::{SlowJob, SlowJobPool};
use hashbrown::{hash_map::Entry, HashMap};
use std::{
    hash::Hash,
    time::{Duration, Instant},
};

enum KeyedJobTask<V> {
    Pending(Instant, Option<SlowJob>),
    Completed(Instant, V),
}

pub struct KeyedJobs<K, V> {
    tx: crossbeam_channel::Sender<(K, V)>,
    rx: crossbeam_channel::Receiver<(K, V)>,
    tasks: HashMap<K, KeyedJobTask<V>>,
    name: &'static str,
    last_gc: Instant,
}

const KEYEDJOBS_GC_INTERVAL: Duration = Duration::from_secs(1);

impl<K: Hash + Eq + Send + Sync + 'static + Clone, V: Send + Sync + 'static> KeyedJobs<K, V> {
    pub fn new(name: &'static str) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self {
            tx,
            rx,
            tasks: HashMap::new(),
            name,
            last_gc: Instant::now(),
        }
    }

    /// Spawn a task on a specified threadpool. The function is given as a thunk
    /// so that if work is needed to create captured variables (e.g.
    /// `Arc::clone`), that only occurs if the task hasn't yet been scheduled.
    pub fn spawn<F: FnOnce(&K) -> V + Send + Sync + 'static>(
        &mut self,
        pool: Option<&SlowJobPool>,
        k: K,
        f: impl FnOnce() -> F,
    ) -> Option<(K, V)> {
        if let Some(pool) = pool {
            while let Ok((k2, v)) = self.rx.try_recv() {
                if k == k2 {
                    return Some((k, v));
                } else {
                    self.tasks
                        .insert(k2, KeyedJobTask::Completed(Instant::now(), v));
                }
            }
            let now = Instant::now();
            if now - self.last_gc > KEYEDJOBS_GC_INTERVAL {
                self.last_gc = now;
                self.tasks.retain(|_, task| match task {
                    KeyedJobTask::Completed(at, _) => now - *at < KEYEDJOBS_GC_INTERVAL,
                    KeyedJobTask::Pending(at, job) => {
                        let fresh = now - *at < KEYEDJOBS_GC_INTERVAL;
                        if !fresh {
                            if let Some(job) = job.take() {
                                // Cancelling a job only fails if the job doesn't exist anymore,
                                // which means that it completed while we tried to GC its pending
                                // struct, which means that we'll GC it in the next cycle, so ignore
                                // the error in this collection.
                                let _ = pool.cancel(job);
                            }
                        }
                        fresh
                    },
                });
            }
            match self.tasks.entry(k.clone()) {
                Entry::Occupied(e) => {
                    let mut ret = None;
                    e.replace_entry_with(|_, v| {
                        if let KeyedJobTask::Completed(_, v) = v {
                            ret = Some((k, v));
                            None
                        } else {
                            Some(v)
                        }
                    });
                    ret
                },
                Entry::Vacant(e) => {
                    // TODO: consider adding a limit to the number of submitted jobs based on the
                    // number of available threads, once SlowJobPool supports a notion of
                    // approximating that
                    let tx = self.tx.clone();
                    let f = f();
                    let job = pool.spawn(self.name, move || {
                        let v = f(&k);
                        let _ = tx.send((k, v));
                    });
                    e.insert(KeyedJobTask::Pending(Instant::now(), Some(job)));
                    None
                },
            }
        } else {
            let v = f()(&k);
            Some((k, v))
        }
    }
}
