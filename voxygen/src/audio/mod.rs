pub mod base;
use base::{Genre, Jukebox};

pub struct AudioFrontend {
    pub(crate) model: Jukebox,
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

        match self.model.player.is_paused() {
            true => match self.model.get_genre() {
                Genre::Bgm => self.model.player.resume(),
                Genre::Sfx => unimplemented!(), // TODO: add support for sound effects.
                Genre::None => (),
            },
            false => self.model.player.load(&path),
        }
    }

    /// Construct in `no-audio` mode for debugging.
    pub(crate) fn no_audio() -> Self {
        Self {
            model: Jukebox::new(Genre::None),
        }
    }
}
