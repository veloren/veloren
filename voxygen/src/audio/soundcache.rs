//! Handles caching and retrieval of decoded `.wav` sfx sound data, eliminating
//! the need to decode files on each playback
use common::assets;
use std::{borrow::Cow, io, sync::Arc};
use tracing::warn;

// Implementation of sound taken from this github issue:
// https://github.com/RustAudio/rodio/issues/141
#[derive(Clone)]
pub struct WavSound(Arc<Vec<u8>>);

impl AsRef<[u8]> for WavSound {
    fn as_ref(&self) -> &[u8] { &self.0 }
}

pub struct SoundLoader;

impl assets::Loader<WavSound> for SoundLoader {
    fn load(content: Cow<[u8]>, _: &str) -> Result<WavSound, assets::BoxedError> {
        let arc = Arc::new(content.into_owned());
        Ok(WavSound(arc))
    }
}

impl assets::Asset for WavSound {
    const EXTENSION: &'static str = "wav";
    type Loader = SoundLoader;

    fn default_value(specifier: &str, error: assets::Error) -> Result<Self, assets::Error> {
        warn!(?specifier, ?error, "Failed to load sound");

        Ok(WavSound::empty())
    }
}

/// Wrapper for decoded audio data
impl WavSound {
    pub fn decoder(self) -> rodio::Decoder<io::Cursor<WavSound>> {
        let cursor = io::Cursor::new(self);
        rodio::Decoder::new(cursor).unwrap()
    }

    /// Returns a `WavSound` containing empty .wav data. This intentionally doesn't
    /// load from the filesystem so we have a reliable fallback when there
    /// is a failure to read a file.
    ///
    /// The data below is the result of passing a very short, silent .wav file
    /// to `Sound::load()`.
    pub fn empty() -> WavSound {
        WavSound(Arc::new(vec![
            82, 73, 70, 70, 40, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0, 1, 0, 1,
            0, 68, 172, 0, 0, 136, 88, 1, 0, 2, 0, 16, 0, 100, 97, 116, 97, 4, 0, 0, 0, 0, 0, 0, 0,
        ]))
    }
}

#[derive(Clone)]
pub struct OggSound(Arc<Vec<u8>>);

impl AsRef<[u8]> for OggSound {
    fn as_ref(&self) -> &[u8] { &self.0 }
}

impl assets::Loader<OggSound> for SoundLoader {
    fn load(content: Cow<[u8]>, _: &str) -> Result<OggSound, assets::BoxedError> {
        let arc = Arc::new(content.into_owned());
        Ok(OggSound(arc))
    }
}

impl assets::Asset for OggSound {
    const EXTENSION: &'static str = "ogg";
    type Loader = SoundLoader;
}

/// Wrapper for decoded audio data
impl OggSound {
    pub fn decoder(self) -> rodio::Decoder<io::Cursor<OggSound>> {
        let cursor = io::Cursor::new(self);
        rodio::Decoder::new(cursor).unwrap()
    }
}