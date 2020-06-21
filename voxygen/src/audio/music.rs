//! Handles music playback and transitions
//!
//! Game music is controlled though a configuration file found in the source at
//! `/assets/voxygen/audio/soundtrack.ron`. Each track enabled in game has a
//! configuration corresponding to the
//! [`SoundtrackItem`](struct.SoundtrackItem.html) format, as well as the
//! corresponding `.ogg` file in the `/assets/voxygen/audio/soundtrack/`
//! directory.
//!
//! If there are errors while reading or deserialising the configuration file, a
//! warning is logged and music will be disabled.
//!
//! ## Adding new music
//!
//! To add a new item, append the details to the audio configuration file, and
//! add the audio file (in `.ogg` format) to the assets directory.
//!
//! The `length` should be provided in seconds. This allows us to know when to
//! transition to another track, without having to spend time determining track
//! length programmatically.
//!
//! An example of a new night time track:
//! ```text
//! (
//!     title: "Sleepy Song",
//!     path: "voxygen.audio.soundtrack.sleepy",
//!     length: 400.0,
//!     timing: Some(Night),
//!     artist: "Elvis",
//! ),
//! ```
//!
//! Before sending an MR for your new track item:
//! - Be conscious of the file size for your new track. Assets contribute to
//!   download sizes
//! - Ensure that the track is mastered to a volume proportionate to other music
//!   tracks
//! - If you are not the author of the track, ensure that the song's licensing
//!   permits usage of the track for non-commercial use
use crate::audio::AudioFrontend;
use common::{assets, state::State};
use rand::{seq::IteratorRandom, thread_rng};
use serde::Deserialize;
use std::time::Instant;
use tracing::warn;

const DAY_START_SECONDS: u32 = 28800; // 8:00
const DAY_END_SECONDS: u32 = 70200; // 19:30

#[derive(Debug, Default, Deserialize)]
struct SoundtrackCollection {
    tracks: Vec<SoundtrackItem>,
}

/// Configuration for a single music track in the soundtrack
#[derive(Debug, Deserialize)]
pub struct SoundtrackItem {
    title: String,
    path: String,
    /// Length of the track in seconds
    length: f64,
    /// Whether this track should play during day or night
    timing: Option<DayPeriod>,
}

/// Allows control over when a track should play based on in-game time of day
#[derive(Debug, Deserialize, PartialEq)]
enum DayPeriod {
    /// 8:00 AM to 7:30 PM
    Day,
    /// 7:31 PM to 6:59 AM
    Night,
}

/// Provides methods to control music playback
pub struct MusicMgr {
    soundtrack: SoundtrackCollection,
    began_playing: Instant,
    next_track_change: f64,
    /// The title of the last track played. Used to prevent a track
    /// being played twice in a row
    last_track: String,
}

impl MusicMgr {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            soundtrack: Self::load_soundtrack_items(),
            began_playing: Instant::now(),
            next_track_change: 0.0,
            last_track: String::from("None"),
        }
    }

    /// Checks whether the previous track has completed. If so, sends a
    /// request to play the next (random) track
    pub fn maintain(&mut self, audio: &mut AudioFrontend, state: &State) {
        if audio.music_enabled()
            && !self.soundtrack.tracks.is_empty()
            && self.began_playing.elapsed().as_secs_f64() > self.next_track_change
        {
            self.play_random_track(audio, state);
        }
    }

    fn play_random_track(&mut self, audio: &mut AudioFrontend, state: &State) {
        const SILENCE_BETWEEN_TRACKS_SECONDS: f64 = 45.0;

        let game_time = (state.get_time_of_day() as u64 % 86400) as u32;
        let current_period_of_day = Self::get_current_day_period(game_time);
        let mut rng = thread_rng();

        let maybe_track = self
            .soundtrack
            .tracks
            .iter()
            .filter(|track| {
                !track.title.eq(&self.last_track)
                    && match &track.timing {
                        Some(period_of_day) => period_of_day == &current_period_of_day,
                        None => true,
                    }
            })
            .choose(&mut rng);

        if let Some(track) = maybe_track {
            self.last_track = String::from(&track.title);
            self.began_playing = Instant::now();
            self.next_track_change = track.length + SILENCE_BETWEEN_TRACKS_SECONDS;

            audio.play_exploration_music(&track.path);
        }
    }

    fn get_current_day_period(game_time: u32) -> DayPeriod {
        if game_time > DAY_START_SECONDS && game_time < DAY_END_SECONDS {
            DayPeriod::Day
        } else {
            DayPeriod::Night
        }
    }

    fn load_soundtrack_items() -> SoundtrackCollection {
        match assets::load_file("voxygen.audio.soundtrack", &["ron"]) {
            Ok(file) => match ron::de::from_reader(file) {
                Ok(config) => config,
                Err(error) => {
                    warn!(
                        "Error parsing music config file, music will not be available: {}",
                        format!("{:#?}", error)
                    );

                    SoundtrackCollection::default()
                },
            },
            Err(error) => {
                warn!(
                    "Error reading music config file, music will not be available: {}",
                    format!("{:#?}", error)
                );

                SoundtrackCollection::default()
            },
        }
    }
}
