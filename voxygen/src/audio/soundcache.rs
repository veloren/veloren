//! Handles caching and retrieval of decoded `.wav` sfx sound data, eliminating
//! the need to decode files on each playback
use common::assets;
use hashbrown::HashMap;
use std::{convert::AsRef, io, io::Read, sync::Arc};
use tracing::warn;

// Implementation of sound taken from this github issue:
// https://github.com/RustAudio/rodio/issues/141
pub struct Sound(Arc<Vec<u8>>);

impl AsRef<[u8]> for Sound {
    fn as_ref(&self) -> &[u8] { &self.0 }
}

/// Wrapper for decoded audio data
impl Sound {
    pub fn load(filename: &str) -> Result<Sound, assets::Error> {
        let mut file = assets::load_file(filename, &["wav"])?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(Sound(Arc::new(buf)))
    }

    pub fn cursor(&self) -> io::Cursor<Sound> { io::Cursor::new(Sound(self.0.clone())) }

    pub fn decoder(&self) -> rodio::Decoder<io::Cursor<Sound>> {
        rodio::Decoder::new(self.cursor()).unwrap()
    }

    /// Returns a `Sound` containing empty .wav data. This intentionally doesn't
    /// load from the filesystem so we have a reliable fallback when there
    /// is a failure to read a file.
    ///
    /// The data below is the result of passing a very short, silent .wav file
    /// to `Sound::load()`.
    pub fn empty() -> Sound {
        Sound(Arc::new(vec![
            82, 73, 70, 70, 40, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0, 1, 0, 1,
            0, 68, 172, 0, 0, 136, 88, 1, 0, 2, 0, 16, 0, 100, 97, 116, 97, 4, 0, 0, 0, 0, 0, 0, 0,
        ]))
    }
}

#[derive(Default)]
pub struct SoundCache {
    sounds: HashMap<String, Sound>,
}

impl SoundCache {
    pub fn load_sound(&mut self, name: &str) -> rodio::Decoder<io::Cursor<Sound>> {
        self.sounds
            .entry(name.to_string())
            .or_insert_with(|| {
                Sound::load(name).unwrap_or_else(|_| {
                    warn!(?name, "SoundCache: Failed to load sound");

                    Sound::empty()
                })
            })
            .decoder()
    }
}
