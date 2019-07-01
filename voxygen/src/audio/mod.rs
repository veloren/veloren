pub mod base;
use base::*;
use crossbeam::{
    channel::{unbounded, Receiver, Sender},
    queue::{PopError, SegQueue},
};
use std::thread;

pub struct AudioFrontend {
    pub(crate) model: Jukebox,
    // mpsc sender and receiver used for audio playback threads.
    //tx_thread: mpsc::Sender,
    //rx_thread: mpsc::Receiver,
}

impl AudioFrontend {
    pub(crate) fn new() -> Self {
        Self {
            model: Jukebox::new(Genre::Bgm),
        }
    }

    /// Play audio.
    pub(crate) fn play(&mut self) {
        let path = base::select_random_music(&Genre::Bgm);

        if self.model.player.is_paused() {
            self.model.player.resume();
        } else {
            self.model.player.load(&path);
        }
    }
}
