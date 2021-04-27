//! Handles caching and retrieval of decoded `.ogg` sfx sound data, eliminating
//! the need to decode files on each playback
use common::assets;
use std::{borrow::Cow, io, sync::Arc};
use tracing::warn;

// Implementation of sound taken from this github issue:
// https://github.com/RustAudio/rodio/issues/141

pub struct SoundLoader;

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
    type Loader = SoundLoader;

    const EXTENSION: &'static str = "ogg";

    fn default_value(specifier: &str, error: assets::Error) -> Result<Self, assets::Error> {
        warn!(?specifier, ?error, "Failed to load sound");

        Ok(OggSound::empty())
    }
}

/// Wrapper for decoded audio data
impl OggSound {
    pub fn decoder(
        self,
    ) -> Result<rodio::Decoder<io::Cursor<OggSound>>, rodio::decoder::DecoderError> {
        let cursor = io::Cursor::new(self);
        rodio::Decoder::new(cursor)
    }

    pub fn empty() -> OggSound {
        OggSound(Arc::new(
            include_bytes!("../../../assets/voxygen/audio/null.ogg").to_vec(),
        ))
    }
}
