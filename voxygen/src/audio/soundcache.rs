//! Handles caching and retrieval of decoded `.ogg` sfx sound data, eliminating
//! the need to decode files on each playback
use common::assets::{self, Loader};
use rodio::{source::Buffered, Decoder, Source};
use std::{borrow::Cow, io, sync::Arc};
use tracing::warn;

// Implementation of sound taken from this github issue:
// https://github.com/RustAudio/rodio/issues/141

pub struct SoundLoader;

#[derive(Clone)]
pub struct OggSound(Buffered<Decoder<io::Cursor<Vec<u8>>>>);

// impl AsRef<[u8]> for OggSound {
//     fn as_ref(&self) -> &[u8] { &self.0 }
// }

impl Loader<OggSound> for SoundLoader {
    fn load(content: Cow<[u8]>, _: &str) -> Result<OggSound, assets::BoxedError> {
        let source = Decoder::new(io::Cursor::new(content.into_owned()))?.buffered();
        Ok(OggSound(source))
    }
}

impl assets::Asset for OggSound {
    type Loader = SoundLoader;

    const EXTENSION: &'static str = "ogg";

    fn default_value(specifier: &str, error: assets::Error) -> Result<Self, assets::Error> {
        warn!(?specifier, ?error, "Failed to load sound");

        Ok(OggSound::empty())
    }
}

/// Wrapper for decoded audio data
impl OggSound {
    pub fn to_source(&self) -> impl Source + Iterator<Item = i16> { self.0.clone() }

    pub fn empty() -> OggSound {
        SoundLoader::load(
            Cow::Borrowed(include_bytes!("../../../assets/voxygen/audio/null.ogg")),
            "empty",
        )
        .unwrap()
    }
}
