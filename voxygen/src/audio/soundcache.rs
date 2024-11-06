//! Handles caching and retrieval of decoded `.ogg` sfx sound data, eliminating
//! the need to decode files on each playback
use common::assets::{self, AssetExt, Loader};
use kira::sound::static_sound::StaticSoundData;
use std::{borrow::Cow, io};
use tracing::warn;

// Implementation of sound taken from this github issue:
// https://github.com/RustAudio/rodio/issues/141

struct SoundLoader;
#[derive(Clone)]
struct OggSound(StaticSoundData);

impl Loader<OggSound> for SoundLoader {
    fn load(content: Cow<[u8]>, _: &str) -> Result<OggSound, assets::BoxedError> {
        let source = StaticSoundData::from_cursor(io::Cursor::new(content.into_owned()))?;
        Ok(OggSound(source))
    }
}

impl assets::Asset for OggSound {
    type Loader = SoundLoader;

    const EXTENSION: &'static str = "ogg";
}

/// Wrapper for decoded audio data
impl OggSound {
    pub fn empty() -> OggSound {
        SoundLoader::load(
            Cow::Borrowed(include_bytes!("../../../assets/voxygen/audio/null.ogg")),
            "ogg",
        )
        .unwrap()
    }
}

#[allow(clippy::implied_bounds_in_impls)]
pub fn load_ogg(specifier: &str) -> StaticSoundData {
    OggSound::load_or_insert_with(specifier, |error| {
        warn!(?specifier, ?error, "Failed to load sound");
        OggSound::empty()
    })
    .cloned()
    .0
}
