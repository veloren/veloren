use hashbrown::HashMap;
use rayon::ThreadPool;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};

/// Provides a Wrapper around rayon threadpool to execute slow-jobs.
/// slow means, the job doesn't need to not complete within the same tick.
/// DO NOT USE I/O blocking jobs, but only CPU heavy jobs.
/// Jobs run here, will reduce the ammount of threads rayon can use during the
/// main tick.
///
/// This Pool allows you to configure certain names of jobs and assign them a
/// maximum number of threads # Example
/// Your system has 16 cores, you assign 12 cores for slow-jobs.
/// Then you can configure all jobs with the name `CHUNK_GENERATOR` to spawn on
/// max 50% (6 = cores) ```rust
/// # use veloren_common::slowjob::SlowJobPool;
/// # use std::sync::Arc;
///
/// let threadpool = rayon::ThreadPoolBuilder::new()
///     .num_threads(16)
///     .build()
///     .unwrap();
/// let pool = SlowJobPool::new(3, Arc::new(threadpool));
/// pool.configure("CHUNK_GENERATOR", |n| n / 2);
/// pool.spawn("CHUNK_GENERATOR", move || println("this is a job"));
/// ```
#[derive(Clone)]
pub struct SlowJobPool {
    internal: Arc<InternalSlowJobPool>,
}

pub struct SlowJob {
    name: String,
    id: u64,
}

struct InternalSlowJobPool {
    next_id: Arc<AtomicU64>,
    queue: RwLock<HashMap<String, HashMap<u64, Queue>>>,
    running_jobs: RwLock<HashMap<String, Arc<AtomicU64>>>,
    configs: RwLock<HashMap<String, Config>>,
    global_running_jobs: Arc<AtomicU64>,
    global_limit: u64,
    threadpool: Arc<ThreadPool>,
}

struct Config {
    max_local: u64,
    spawned_total: Arc<AtomicU64>,
}

struct Queue {
    task: Box<dyn FnOnce() + Send + Sync + 'static>,
    spawned_total: Arc<AtomicU64>,
    local_running_jobs: Arc<AtomicU64>,
}

impl InternalSlowJobPool {
    pub fn new(global_limit: u64, threadpool: Arc<ThreadPool>) -> Self {
        Self {
            next_id: Arc::new(AtomicU64::new(0)),
            queue: RwLock::new(HashMap::new()),
            running_jobs: RwLock::new(HashMap::new()),
            configs: RwLock::new(HashMap::new()),
            global_running_jobs: Arc::new(AtomicU64::new(0)),
            global_limit,
            threadpool,
        }
    }

    fn maintain(&self) {
        let jobs_available = self.global_limit - self.global_running_jobs.load(Ordering::Relaxed);
        if jobs_available == 0 {
            // we run at limit, can't spawn
            return;
        }
        let possible = {
            let lock = self.queue.read().unwrap();
            lock.iter()
                .map(|(name, queues)| {
                    if !queues.is_empty() {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .flatten()
                .collect::<Vec<_>>()
        };

        let mut possible_total = {
            let mut possible = possible;
            let lock = self.configs.read().unwrap();
            possible
                .drain(..)
                .map(|name| {
                    let c = lock.get(&name).unwrap();
                    (
                        name,
                        c.spawned_total.load(Ordering::Relaxed) / c.max_local,
                        c.max_local,
                    )
                })
                .collect::<Vec<_>>()
        };
        possible_total.sort_by_key(|(_, i, _)| *i);

        let mut lock = self.queue.write().unwrap();
        for i in 0..jobs_available as usize {
            if let Some((name, _, max)) = possible_total.get(i) {
                if let Some(map) = lock.get_mut(name) {
                    let firstkey = match map.keys().next() {
                        Some(k) => *k,
                        None => continue,
                    };

                    if let Some(queue) = map.remove(&firstkey) {
                        if queue.local_running_jobs.load(Ordering::Relaxed) < *max {
                            self.fire(queue);
                        } else {
                            map.insert(firstkey, queue);
                        }
                    }
                }
            }
        }
    }

    fn fire(&self, queue: Queue) {
        queue.spawned_total.fetch_add(1, Ordering::Relaxed);
        queue.local_running_jobs.fetch_add(1, Ordering::Relaxed);
        self.global_running_jobs.fetch_add(1, Ordering::Relaxed);
        self.threadpool.spawn(queue.task);
    }
}

impl SlowJobPool {
    pub fn new(global_limit: u64, threadpool: Arc<ThreadPool>) -> Self {
        Self {
            internal: Arc::new(InternalSlowJobPool::new(global_limit, threadpool)),
        }
    }

    /// configure a NAME to spawn up to f(n) threads, depending on how many
    /// threads we globally have available
    pub fn configure<F>(&self, name: &str, f: F)
    where
        F: Fn(u64) -> u64,
    {
        let cnf = Config {
            max_local: f(self.internal.global_limit),
            spawned_total: Arc::new(AtomicU64::new(0)),
        };
        let mut lock = self.internal.configs.write().unwrap();
        lock.insert(name.to_string(), cnf);
    }

    /// spawn a new slow job on a certain NAME
    pub fn spawn<F>(&self, name: &str, f: F) -> SlowJob
    where
        F: FnOnce() + Send + Sync + 'static,
    {
        let id = self.internal.next_id.fetch_add(1, Ordering::Relaxed);
        self.internal
            .queue
            .write()
            .unwrap()
            .entry(name.to_string())
            .or_default()
            .insert(id, self.queue(name, f));
        self.maintain();
        SlowJob {
            name: name.to_string(),
            id,
        }
    }

    fn queue<F>(&self, name: &str, f: F) -> Queue
    where
        F: FnOnce() + Send + Sync + 'static,
    {
        let internal = Arc::clone(&self.internal);
        let spawned_total = Arc::clone(
            &self
                .internal
                .configs
                .read()
                .unwrap()
                .get(name)
                .expect("can't spawn a non-configued slowjob")
                .spawned_total,
        );
        let local_running_jobs_clone = {
            let mut lock = self.internal.running_jobs.write().unwrap();
            Arc::clone(&lock.entry(name.to_string()).or_default())
        };
        let local_running_jobs = Arc::clone(&local_running_jobs_clone);
        let global_running_jobs_clone = Arc::clone(&self.internal.global_running_jobs);
        let _name_clones = name.to_string();
        Queue {
            task: Box::new(move || {
                common_base::prof_span!(_guard, &_name_clones);
                f();
                local_running_jobs_clone.fetch_sub(1, Ordering::Relaxed);
                global_running_jobs_clone.fetch_sub(1, Ordering::Relaxed);
                // directly maintain the next task afterwards
                internal.maintain();
            }),
            spawned_total,
            local_running_jobs,
        }
    }

    pub fn cancel(&self, job: SlowJob) {
        let mut lock = self.internal.queue.write().unwrap();
        if let Some(map) = lock.get_mut(&job.name) {
            map.remove(&job.id);
        }
    }

    fn maintain(&self) { self.internal.maintain() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::Mutex,
        time::{Duration, Instant},
    };

    fn mock_fn(
        name: &str,
        start_time: &Arc<Mutex<Option<Instant>>>,
        done: &Arc<AtomicU64>,
    ) -> impl FnOnce() {
        let name = name.to_string();
        let start_time = Arc::clone(start_time);
        let done = Arc::clone(done);
        move || {
            println!("Start {}", name);
            *start_time.lock().unwrap() = Some(Instant::now());
            std::thread::sleep(Duration::from_millis(500));
            done.fetch_add(1, Ordering::Relaxed);
            println!("Finished {}", name);
        }
    }

    #[test]
    fn global_limit() {
        let threadpool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let pool = SlowJobPool::new(3, Arc::new(threadpool));
        pool.configure("FOO", |_| 1000);
        let start = Instant::now();
        let f1 = Arc::new(Mutex::new(None));
        let f2 = Arc::new(Mutex::new(None));
        let f3 = Arc::new(Mutex::new(None));
        let f4 = Arc::new(Mutex::new(None));
        let f5 = Arc::new(Mutex::new(None));
        let f6 = Arc::new(Mutex::new(None));
        let f7 = Arc::new(Mutex::new(None));
        let done = Arc::new(AtomicU64::new(0));
        pool.spawn("FOO", mock_fn("foo1", &f1, &done));
        pool.spawn("FOO", mock_fn("foo2", &f2, &done));
        pool.spawn("FOO", mock_fn("foo3", &f3, &done));
        std::thread::sleep(Duration::from_millis(300));
        pool.spawn("FOO", mock_fn("foo4", &f4, &done));
        pool.spawn("FOO", mock_fn("foo5", &f5, &done));
        pool.spawn("FOO", mock_fn("foo6", &f6, &done));
        std::thread::sleep(Duration::from_millis(300));
        pool.spawn("FOO", mock_fn("foo7", &f7, &done));
        std::thread::sleep(Duration::from_secs(1));
        let measure = |a: Arc<Mutex<Option<Instant>>>, s: Instant| {
            a.lock().unwrap().unwrap().duration_since(s).as_millis()
        };
        let f1 = measure(f1, start);
        let f2 = measure(f2, start);
        let f3 = measure(f3, start);
        let f4 = measure(f4, start);
        let f5 = measure(f5, start);
        let f6 = measure(f6, start);
        let f7 = measure(f7, start);
        assert_eq!(done.load(Ordering::Relaxed), 7);
        assert!(f1 < 500);
        assert!(f2 < 500);
        assert!(f3 < 500);
        assert!(f4 < 1000);
        assert!(f5 < 1000);
        assert!(f6 < 1000);
        assert!(f7 < 1500);
    }

    #[test]
    fn local_limit() {
        let threadpool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let pool = SlowJobPool::new(100, Arc::new(threadpool));
        pool.configure("FOO", |_| 3);
        let start = Instant::now();
        let f1 = Arc::new(Mutex::new(None));
        let f2 = Arc::new(Mutex::new(None));
        let f3 = Arc::new(Mutex::new(None));
        let f4 = Arc::new(Mutex::new(None));
        let f5 = Arc::new(Mutex::new(None));
        let f6 = Arc::new(Mutex::new(None));
        let f7 = Arc::new(Mutex::new(None));
        let done = Arc::new(AtomicU64::new(0));
        pool.spawn("FOO", mock_fn("foo1", &f1, &done));
        pool.spawn("FOO", mock_fn("foo2", &f2, &done));
        pool.spawn("FOO", mock_fn("foo3", &f3, &done));
        std::thread::sleep(Duration::from_millis(300));
        pool.spawn("FOO", mock_fn("foo4", &f4, &done));
        pool.spawn("FOO", mock_fn("foo5", &f5, &done));
        pool.spawn("FOO", mock_fn("foo6", &f6, &done));
        std::thread::sleep(Duration::from_millis(300));
        pool.spawn("FOO", mock_fn("foo7", &f7, &done));
        std::thread::sleep(Duration::from_secs(1));
        let measure = |a: Arc<Mutex<Option<Instant>>>, s: Instant| {
            a.lock().unwrap().unwrap().duration_since(s).as_millis()
        };
        let f1 = measure(f1, start);
        let f2 = measure(f2, start);
        let f3 = measure(f3, start);
        let f4 = measure(f4, start);
        let f5 = measure(f5, start);
        let f6 = measure(f6, start);
        let f7 = measure(f7, start);
        assert_eq!(done.load(Ordering::Relaxed), 7);
        assert!(f1 < 500);
        assert!(f2 < 500);
        assert!(f3 < 500);
        assert!(f4 < 1000);
        assert!(f5 < 1000);
        assert!(f6 < 1000);
        assert!(f7 < 1500);
    }

    #[test]
    fn pool() {
        let threadpool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let pool = SlowJobPool::new(2, Arc::new(threadpool));
        pool.configure("FOO", |n| n);
        pool.configure("BAR", |n| n / 2);
        let start = Instant::now();
        let f1 = Arc::new(Mutex::new(None));
        let f2 = Arc::new(Mutex::new(None));
        let b1 = Arc::new(Mutex::new(None));
        let b2 = Arc::new(Mutex::new(None));
        let done = Arc::new(AtomicU64::new(0));
        pool.spawn("FOO", mock_fn("foo1", &f1, &done));
        pool.spawn("FOO", mock_fn("foo2", &f2, &done));
        std::thread::sleep(Duration::from_millis(1000));
        pool.spawn("BAR", mock_fn("bar1", &b1, &done));
        pool.spawn("BAR", mock_fn("bar2", &b2, &done));
        std::thread::sleep(Duration::from_secs(2));
        let measure = |a: Arc<Mutex<Option<Instant>>>, s: Instant| {
            a.lock().unwrap().unwrap().duration_since(s).as_millis()
        };
        let f1 = measure(f1, start);
        let f2 = measure(f2, start);
        let b1 = measure(b1, start);
        let b2 = measure(b2, start);
        // Expect:
        //  [F1, F2]
        //  [B1]
        //  [B2]
        assert_eq!(done.load(Ordering::Relaxed), 4);
        assert!(f1 < 500);
        assert!(f2 < 500);
        println!("b1 {}", b1);
        println!("b2 {}", b2);
        assert!((1000..1500).contains(&b1));
        assert!((1500..2000).contains(&b2));
    }
}
