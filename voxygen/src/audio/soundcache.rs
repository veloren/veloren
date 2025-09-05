//! Handles caching and retrieval of decoded `.ogg` sfx sound data, eliminating
//! the need to decode files on each playback
use common::assets::{AssetExt, BoxedError, FileAsset};
use kira::{
    Decibels, StartTime, Tween, Value,
    sound::{
        FromFileError, IntoOptionalRegion, PlaybackState, SoundData,
        static_sound::{StaticSoundData, StaticSoundHandle},
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
};
use std::{
    borrow::Cow,
    io::{self, Cursor},
    sync::Arc,
};
use tracing::warn;

// Kira does not provide a generic interface over sound data and sound handles,
// but we want to use both streaming and static sound data for music and sfx.
//
// To work around this, here's a small wrapper exposing the functionality for
// both audio data types.

pub enum AnySoundData {
    Static(StaticSoundData),
    Streaming(StreamingSoundData<FromFileError>),
}

#[derive(Debug)]
pub enum AnySoundError {
    Static(<StaticSoundData as SoundData>::Error),
    Streaming(<StreamingSoundData<FromFileError> as SoundData>::Error),
}

impl SoundData for AnySoundData {
    type Error = AnySoundError;
    type Handle = AnySoundHandle;

    fn into_sound(self) -> Result<(Box<dyn kira::sound::Sound>, Self::Handle), Self::Error> {
        match self {
            AnySoundData::Static(data) => <StaticSoundData as SoundData>::into_sound(data)
                .map(|(sound, handle)| (sound, AnySoundHandle::Static(handle)))
                .map_err(AnySoundError::Static),
            AnySoundData::Streaming(data) => {
                <StreamingSoundData<FromFileError> as SoundData>::into_sound(data)
                    .map(|(sound, handle)| (sound, AnySoundHandle::Streaming(handle)))
                    .map_err(AnySoundError::Streaming)
            },
        }
    }
}

impl AnySoundData {
    pub fn fade_in_tween(self, fade_in_tween: impl Into<Option<Tween>>) -> Self {
        match self {
            AnySoundData::Static(d) => AnySoundData::Static(d.fade_in_tween(fade_in_tween)),
            AnySoundData::Streaming(d) => AnySoundData::Streaming(d.fade_in_tween(fade_in_tween)),
        }
    }

    pub fn start_time(self, start_time: impl Into<StartTime>) -> Self {
        match self {
            AnySoundData::Static(d) => AnySoundData::Static(d.start_time(start_time)),
            AnySoundData::Streaming(d) => AnySoundData::Streaming(d.start_time(start_time)),
        }
    }

    pub fn volume(self, volume: impl Into<Value<Decibels>>) -> Self {
        match self {
            AnySoundData::Static(d) => AnySoundData::Static(d.volume(volume)),
            AnySoundData::Streaming(d) => AnySoundData::Streaming(d.volume(volume)),
        }
    }

    pub fn loop_region(self, loop_region: impl IntoOptionalRegion) -> Self {
        match self {
            AnySoundData::Static(d) => AnySoundData::Static(d.loop_region(loop_region)),
            AnySoundData::Streaming(d) => AnySoundData::Streaming(d.loop_region(loop_region)),
        }
    }
}

#[derive(Debug)]
pub enum AnySoundHandle {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<FromFileError>),
}

impl AnySoundHandle {
    pub fn state(&self) -> PlaybackState {
        match self {
            AnySoundHandle::Static(h) => h.state(),
            AnySoundHandle::Streaming(h) => h.state(),
        }
    }

    pub fn position(&self) -> f64 {
        match self {
            AnySoundHandle::Static(h) => h.position(),
            AnySoundHandle::Streaming(h) => h.position(),
        }
    }

    pub fn set_volume(&mut self, volume: impl Into<Value<Decibels>>, tween: Tween) {
        match self {
            AnySoundHandle::Static(h) => h.set_volume(volume, tween),
            AnySoundHandle::Streaming(h) => h.set_volume(volume, tween),
        }
    }

    pub fn stop(&mut self, tween: Tween) {
        match self {
            AnySoundHandle::Static(h) => h.stop(tween),
            AnySoundHandle::Streaming(h) => h.stop(tween),
        }
    }

    pub fn set_loop_region(&mut self, loop_region: impl IntoOptionalRegion) {
        match self {
            AnySoundHandle::Static(h) => h.set_loop_region(loop_region),
            AnySoundHandle::Streaming(h) => h.set_loop_region(loop_region),
        }
    }
}

#[derive(Clone)]
struct OggSound(StaticSoundData);

#[derive(Clone)]
struct StreamedOggSound(Arc<[u8]>);

impl FileAsset for OggSound {
    const EXTENSION: &'static str = "ogg";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        let source = StaticSoundData::from_cursor(io::Cursor::new(bytes.into_owned()))?;
        Ok(OggSound(source))
    }
}

impl FileAsset for StreamedOggSound {
    const EXTENSION: &'static str = "ogg";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        // Store the raw file contents to be streamed later
        Ok(StreamedOggSound(Arc::from(bytes)))
    }
}

/// Wrapper for decoded audio data
impl OggSound {
    pub fn empty() -> OggSound {
        OggSound::from_bytes(Cow::Borrowed(include_bytes!(
            "../../../assets/voxygen/audio/null.ogg"
        )))
        .unwrap()
    }
}

pub fn load_ogg(specifier: &str, streamed: bool) -> AnySoundData {
    if streamed {
        match StreamedOggSound::load(specifier) {
            Ok(handle) => StreamingSoundData::from_cursor(Cursor::new(handle.cloned().0))
                .map_or_else(
                    |error| {
                        warn!(?error, "Error while creating streaming sound data");
                        AnySoundData::Static(OggSound::empty().0)
                    },
                    AnySoundData::Streaming,
                ),

            Err(error) => {
                warn!(?specifier, ?error, "Failed to load sound");
                AnySoundData::Static(OggSound::empty().0)
            },
        }
    } else {
        AnySoundData::Static(
            OggSound::load_or_insert_with(specifier, |error| {
                warn!(?specifier, ?error, "Failed to load sound");
                OggSound::empty()
            })
            .cloned()
            .0,
        )
    }
}
