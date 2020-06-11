use common::assets;
use hashbrown::HashMap;
use std::{convert::AsRef, io, io::Read, sync::Arc};

// Implementation of sound taken from this github issue:
// https://github.com/RustAudio/rodio/issues/141
pub struct Sound(Arc<Vec<u8>>);

impl AsRef<[u8]> for Sound {
    fn as_ref(&self) -> &[u8] { &self.0 }
}

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
}

pub struct SoundCache {
    sounds: HashMap<String, Sound>,
}

impl SoundCache {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            sounds: HashMap::new(),
        }
    }

    pub fn load_sound(&mut self, name: &str) -> rodio::Decoder<io::Cursor<Sound>> {
        self.sounds
            .entry(name.to_string())
            .or_insert(Sound::load(name).unwrap())
            .decoder()
    }
}
