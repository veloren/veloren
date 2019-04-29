use std::thread;

#[derive(Debug, Clone)]
/*only have one JobManager per System because it bounds to all threads*/
pub struct JobManager {

}

impl JobManager{
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn repeat<F>(&self, mut f: F) where F: FnMut() -> bool, F: Send + 'static {
        let worker = thread::spawn(move || {
            while f() {

            }
        });
    }
}