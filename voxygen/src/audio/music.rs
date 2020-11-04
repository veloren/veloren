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
//!     biome: Some(Forest),
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
use client::Client;
use common::{
    assets,
    state::State,
    terrain::{BiomeKind, SitesKind},
};
use rand::{prelude::SliceRandom, thread_rng};
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
    biomes: BiomeProbability,
    site: Option<SitesKind>,
}

#[derive(Debug, Deserialize)]
pub struct BiomeProbability {
    void: u8,
    lake: u8,
    grassland: u8,
    ocean: u8,
    mountain: u8,
    snowland: u8,
    desert: u8,
    swamp: u8,
    jungle: u8,
    forest: u8,
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
    soundtrack: SoundtrackCollection,
    began_playing: Instant,
    began_fading: Instant,
    next_track_change: f64,
    /// The title of the last track played. Used to prevent a track
    /// being played twice in a row
    last_track: String,
    last_biome: BiomeKind,
    playing: PlayState,
}

impl MusicMgr {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            soundtrack: Self::load_soundtrack_items(),
            began_playing: Instant::now(),
            began_fading: Instant::now(),
            next_track_change: 0.0,
            last_track: String::from("None"),
            last_biome: BiomeKind::Void,
            playing: PlayState::Stopped,
        }
    }

    /// Checks whether the previous track has completed. If so, sends a
    /// request to play the next (random) track
    pub fn maintain(&mut self, audio: &mut AudioFrontend, state: &State, client: &Client) {
        // Gets the current player biome
        //let current_biome: BiomeKind = match client.current_chunk() {
        //    Some(chunk) => chunk.meta().biome(),
        //    _ => self.last_biome,
        //};

        if let Some(current_chunk) = client.current_chunk() {
            println!("biome: {:?}", current_chunk.meta().biome());
            println!("chaos: {}", current_chunk.meta().chaos());
            println!("alt: {}", current_chunk.meta().alt());
            println!("temp: {}", current_chunk.meta().temp());
            println!("tree_density: {}", current_chunk.meta().tree_density());
            println!("humidity: {}", current_chunk.meta().humidity());
            println!("cave_alt: {}", current_chunk.meta().cave_alt());
            if let Some(position) = client.current_position() {
                println!("player_alt: {}", position[2]);
            }
        }

        if audio.music_enabled()
            && !self.soundtrack.tracks.is_empty()
            && self.began_playing.elapsed().as_secs_f64() > self.next_track_change
        //        || self.playing == PlayState::Stopped)
        //    && self.playing != PlayState::FadingOut
        {
            self.play_random_track(audio, state, client);
        //    self.playing = PlayState::Playing;
        //} else if current_biome != self.last_biome && self.playing == PlayState::Playing {
        //    audio.fade_out_exploration_music();
        //    self.began_fading = Instant::now();
        //    self.playing = PlayState::FadingOut;
        //} else if self.began_fading.elapsed().as_secs_f64() > 5.0
        //    && self.playing == PlayState::FadingOut
        //{
        //    audio.stop_exploration_music();
        //    self.playing = PlayState::Stopped;
        }
        //self.last_biome = current_biome;
    }

    fn play_random_track(&mut self, audio: &mut AudioFrontend, state: &State, client: &Client) {
        //const SILENCE_BETWEEN_TRACKS_SECONDS: f64 = 45.0;
        const SILENCE_BETWEEN_TRACKS_SECONDS: f64 = 5.0;

        let game_time = (state.get_time_of_day() as u64 % 86400) as u32;
        let current_period_of_day = Self::get_current_day_period(game_time);
        let current_biome = Self::get_current_biome(client);
        let current_site = Self::get_current_site(client);
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
            .filter(|track| match &track.site {
                Some(site) => site == &current_site,
                None => true,
            })
            .filter(|track| match current_biome {
                BiomeKind::Void => false,
                BiomeKind::Lake => track.biomes.lake > 0,
                BiomeKind::Grassland => track.biomes.grassland > 0,
                BiomeKind::Ocean => track.biomes.ocean > 0,
                BiomeKind::Mountain => track.biomes.mountain > 0,
                BiomeKind::Snowland => track.biomes.snowland > 0,
                BiomeKind::Desert => track.biomes.desert > 0,
                BiomeKind::Swamp => track.biomes.swamp > 0,
                BiomeKind::Jungle => track.biomes.jungle > 0,
                BiomeKind::Forest => track.biomes.forest > 0,
            })
            .collect::<Vec<&SoundtrackItem>>();

        //let new_maybe_track = maybe_track
        //    .choose_weighted(&mut rng, |track|
        // track.biomes.unwrap().entry(current_biome));

        let new_maybe_track = maybe_track.choose_weighted(&mut rng, |track| match current_biome {
            BiomeKind::Void => track.biomes.void,
            BiomeKind::Lake => track.biomes.lake,
            BiomeKind::Grassland => track.biomes.grassland,
            BiomeKind::Ocean => track.biomes.ocean,
            BiomeKind::Mountain => track.biomes.mountain,
            BiomeKind::Snowland => track.biomes.snowland,
            BiomeKind::Desert => track.biomes.desert,
            BiomeKind::Swamp => track.biomes.swamp,
            BiomeKind::Jungle => track.biomes.jungle,
            BiomeKind::Forest => track.biomes.forest,
        });

        if let Ok(track) = new_maybe_track {
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

    fn get_current_biome(client: &Client) -> BiomeKind {
        match client.current_chunk() {
            Some(chunk) => chunk.meta().biome(),
            _ => BiomeKind::Void,
        }
    }

    fn get_current_site(client: &Client) -> SitesKind {
        let mut player_alt = 0.0;
        if let Some(position) = client.current_position() {
            player_alt = position[2];
        }
        let mut cave_alt = 0.0;
        let mut alt = 0.0;
        if let Some(chunk) = client.current_chunk() {
            alt = chunk.meta().alt();
            cave_alt = chunk.meta().cave_alt();
        }
        if player_alt < cave_alt && cave_alt != 0.0 {
            SitesKind::Cave
        } else if player_alt < (alt - 30.0) {
            SitesKind::Dungeon
        } else {
            SitesKind::None
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
