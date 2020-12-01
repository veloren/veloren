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
//!     biomes: [
//!         (Forest, 1),
//!         (Grassland, 2),
//!     ],
//!     site: None,
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
use crate::audio::{AudioFrontend, MusicChannelTag};
use client::Client;
use common::{
    assets,
    terrain::{BiomeKind, SitesKind},
};
use common_sys::state::State;
use rand::{prelude::SliceRandom, thread_rng, Rng};
use serde::Deserialize;
use std::time::Instant;
use tracing::warn;

// TODO These should eventually not be constants if we have seasons
const DAY_START_SECONDS: u32 = 28800; // 8:00
const DAY_END_SECONDS: u32 = 70200; // 19:30

/// Collection of all the tracks
#[derive(Debug, Default, Deserialize)]
struct SoundtrackCollection {
    /// List of tracks
    tracks: Vec<SoundtrackItem>,
}

/// Configuration for a single music track in the soundtrack
#[derive(Debug, Deserialize)]
pub struct SoundtrackItem {
    /// Song title
    title: String,
    /// File path to asset
    path: String,
    /// Length of the track in seconds
    length: f32,
    /// Whether this track should play during day or night
    timing: Option<DayPeriod>,
    /// What biomes this track should play in with chance of play
    biomes: Vec<(BiomeKind, u8)>,
    /// Whether this track should play in a specific site
    site: Option<SitesKind>,
}

/// Allows control over when a track should play based on in-game time of day
#[derive(Debug, Deserialize, PartialEq)]
enum DayPeriod {
    /// 8:00 AM to 7:30 PM
    Day,
    /// 7:31 PM to 6:59 AM
    Night,
}

/// Determines whether the sound is stopped, playing, or fading
#[derive(Debug, Deserialize, PartialEq)]
enum PlayState {
    Playing,
    Stopped,
    FadingOut,
    FadingIn,
}

/// Provides methods to control music playback
pub struct MusicMgr {
    /// Collection of all the tracks
    soundtrack: SoundtrackCollection,
    /// Instant at which the current track began playing
    began_playing: Instant,
    /// Time until the next track should be played
    next_track_change: f32,
    /// The title of the last track played. Used to prevent a track
    /// being played twice in a row
    last_track: String,
}

impl Default for MusicMgr {
    fn default() -> Self {
        Self {
            soundtrack: Self::load_soundtrack_items(),
            began_playing: Instant::now(),
            next_track_change: 0.0,
            last_track: String::from("None"),
        }
    }
}

impl MusicMgr {
    /// Checks whether the previous track has completed. If so, sends a
    /// request to play the next (random) track
    pub fn maintain(&mut self, audio: &mut AudioFrontend, state: &State, client: &Client) {
        //if let Some(current_chunk) = client.current_chunk() {
        //println!("biome: {:?}", current_chunk.meta().biome());
        //println!("chaos: {}", current_chunk.meta().chaos());
        //println!("alt: {}", current_chunk.meta().alt());
        //println!("tree_density: {}",
        // current_chunk.meta().tree_density());
        //if let Some(position) = client.current::<comp::Pos>() {
        //    player_alt = position.0.z;
        //}

        if audio.music_enabled()
            && !self.soundtrack.tracks.is_empty()
            && self.began_playing.elapsed().as_secs_f32() > self.next_track_change
        {
            self.play_random_track(audio, state, client);
        }
    }

    fn play_random_track(&mut self, audio: &mut AudioFrontend, state: &State, client: &Client) {
        let mut rng = thread_rng();

        // Adds a bit of randomness between plays
        let silence_between_tracks_seconds: f32 = rng.gen_range(45.0, 120.0);

        let game_time = (state.get_time_of_day() as u64 % 86400) as u32;
        let current_period_of_day = Self::get_current_day_period(game_time);
        let current_biome = client.current_biome();
        let current_site = client.current_site();

        // Filters out tracks not matching the timing, site, and biome
        let maybe_tracks = self
            .soundtrack
            .tracks
            .iter()
            .filter(|track| {
                !track.title.eq(&self.last_track)
                    && match &track.timing {
                        Some(period_of_day) => period_of_day == &current_period_of_day,
                        None => true,
                    }
                    && match &track.site {
                        Some(site) => site == &current_site,
                        None => true,
                    }
            })
            .filter(|track| {
                let mut result = false;
                if !track.biomes.is_empty() {
                    for biome in track.biomes.iter() {
                        if biome.0 == current_biome {
                            result = true;
                        }
                    }
                } else {
                    result = true;
                }
                result
            })
            .collect::<Vec<&SoundtrackItem>>();

        // Randomly selects a track from the remaining tracks weighted based
        // on the biome
        let new_maybe_track = maybe_tracks.choose_weighted(&mut rng, |track| {
            let mut chance = 0;
            if !track.biomes.is_empty() {
                for biome in track.biomes.iter() {
                    if biome.0 == current_biome {
                        chance = biome.1;
                    }
                }
            } else {
                // If no biome is listed, the song is still added to the
                // rotation to allow for site specific songs to play
                // in any biome
                chance = 1;
            }
            chance
        });

        if let Ok(track) = new_maybe_track {
            //println!("Now playing {:?}", track.title);
            self.last_track = String::from(&track.title);
            self.began_playing = Instant::now();
            self.next_track_change = track.length + silence_between_tracks_seconds;

            audio.play_music(&track.path, MusicChannelTag::Exploration);
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
