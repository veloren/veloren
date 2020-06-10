use crossbeam::channel::{select, unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use log::warn;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
    thread,
};

type Handler = Box<dyn Fn() + Send>;

lazy_static! {
    static ref WATCHER_TX: Mutex<Sender<(PathBuf, Handler, Weak<AtomicBool>)>> =
        Mutex::new(Watcher::new().run());
}

// This will need to be adjusted when specifier mapping to asset location
// becomes more dynamic
struct Watcher {
    watching: HashMap<PathBuf, (Handler, Vec<Weak<AtomicBool>>)>,
    watcher: RecommendedWatcher,
    event_rx: Receiver<Result<Event, notify::Error>>,
}
impl Watcher {
    fn new() -> Self {
        let (event_tx, event_rx) = unbounded();
        Watcher {
            watching: HashMap::new(),
            watcher: notify::Watcher::new_immediate(move |event| {
                let _ = event_tx.send(event);
            })
            .expect("Failed to create notify::Watcher"),
            event_rx,
        }
    }

    fn watch(&mut self, path: PathBuf, handler: Handler, signal: Weak<AtomicBool>) {
        match self.watching.get_mut(&path) {
            Some((_, ref mut v)) => {
                if !v.iter().any(|s| match (s.upgrade(), signal.upgrade()) {
                    (Some(arc1), Some(arc2)) => Arc::ptr_eq(&arc1, &arc2),
                    _ => false,
                }) {
                    v.push(signal);
                }
            },
            None => {
                if let Err(err) = self.watcher.watch(path.clone(), RecursiveMode::Recursive) {
                    warn!("Could not start watching {:#?} due to: {}", &path, err);
                    return;
                }
                self.watching.insert(path, (handler, vec![signal]));
            },
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Event {
            kind: EventKind::Modify(_),
            paths,
            ..
        } = event
        {
            for path in paths {
                match self.watching.get_mut(&path) {
                    Some((reloader, ref mut signals)) => {
                        if !signals.is_empty() {
                            // Reload this file
                            reloader();

                            signals.retain(|signal| match signal.upgrade() {
                                Some(signal) => {
                                    signal.store(true, Ordering::Release);
                                    true
                                },
                                None => false,
                            });
                        }
                        // If there is no one to signal stop watching this path
                        if signals.is_empty() {
                            if let Err(err) = self.watcher.unwatch(&path) {
                                warn!("Error unwatching: {}", err);
                            }
                            self.watching.remove(&path);
                        }
                    },
                    None => {
                        warn!(
                            "Watching {:#?} but there are no signals for this path. The path will \
                             be unwatched.",
                            path
                        );
                        if let Err(err) = self.watcher.unwatch(&path) {
                            warn!("Error unwatching: {}", err);
                        }
                    },
                }
            }
        }
    }

    #[allow(clippy::drop_copy)] // TODO: Pending review in #587
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    #[allow(clippy::zero_ptr)] // TODO: Pending review in #587
    fn run(mut self) -> Sender<(PathBuf, Handler, Weak<AtomicBool>)> {
        let (watch_tx, watch_rx) = unbounded();

        thread::spawn(move || {
            loop {
                select! {
                    recv(watch_rx) -> res => match res {
                        Ok((path, handler, signal)) => self.watch(path, handler, signal),
                        // Disconnected
                        Err(_) => (),
                    },
                    recv(self.event_rx) -> res => match res {
                        Ok(Ok(event)) => self.handle_event(event),
                        // Notify Error
                        Ok(Err(err)) => error!("Notify error: {}", err),
                        // Disconnected
                        Err(_) => (),
                    },
                }
            }
        });

        watch_tx
    }
}

pub struct ReloadIndicator {
    reloaded: Arc<AtomicBool>,
    // Paths that have already been added
    paths: Vec<PathBuf>,
}
impl ReloadIndicator {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            reloaded: Arc::new(AtomicBool::new(false)),
            paths: Vec::new(),
        }
    }

    pub fn add<F>(&mut self, path: PathBuf, reloader: F)
    where
        F: 'static + Fn() + Send,
    {
        // Check to see if this was already added
        if self.paths.iter().any(|p| *p == path) {
            // Nothing else needs to be done
            return;
        } else {
            self.paths.push(path.clone());
        };

        if WATCHER_TX
            .lock()
            .unwrap()
            .send((path, Box::new(reloader), Arc::downgrade(&self.reloaded)))
            .is_err()
        {
            error!("Could not add. Asset watcher channel disconnected.");
        }
    }

    // Returns true if the watched file was changed
    pub fn reloaded(&self) -> bool { self.reloaded.swap(false, Ordering::Acquire) }
}
