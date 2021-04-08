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
//!     activity: Explore,
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
    assets::{self, AssetExt, AssetHandle},
    terrain::{BiomeKind, SitesKind},
};
use common_sys::state::State;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use rand::{prelude::SliceRandom, thread_rng, Rng};
use serde::Deserialize;
use std::time::Instant;
use tracing::{debug, trace, warn};

/// Collection of all the tracks
#[derive(Debug, Deserialize)]
struct SoundtrackCollection<T> {
    /// List of tracks
    tracks: Vec<T>,
}

impl<T> Default for SoundtrackCollection<T> {
    fn default() -> Self { Self { tracks: Vec::new() } }
}

/// Configuration for a single music track in the soundtrack
#[derive(Clone, Debug, Deserialize)]
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
    /// What the player is doing when the track is played (i.e. exploring,
    /// combat)
    activity: MusicActivity,
    /// What activity to override the activity state with, if any (e.g. to make
    /// a long combat intro also act like the loop for the purposes of outro
    /// transitions)
    #[serde(default)]
    activity_override: Option<MusicActivityState>,
}

#[derive(Clone, Debug, Deserialize)]
enum RawSoundtrackItem {
    Individual(SoundtrackItem),
    Segmented {
        title: String,
        timing: Option<DayPeriod>,
        biomes: Vec<(BiomeKind, u8)>,
        site: Option<SitesKind>,
        segments: Vec<(String, f32, MusicActivity, Option<MusicActivityState>)>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
enum CombatIntensity {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
enum MusicActivityState {
    Explore,
    Combat(CombatIntensity),
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
enum MusicActivity {
    State(MusicActivityState),
    Transition(MusicActivityState, MusicActivityState),
}

/// Allows control over when a track should play based on in-game time of day
#[derive(Clone, Debug, Deserialize, PartialEq)]
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
    soundtrack: AssetHandle<SoundtrackCollection<SoundtrackItem>>,
    /// Instant at which the current track began playing
    began_playing: Instant,
    /// Time until the next track should be played
    next_track_change: f32,
    /// The title of the last track played. Used to prevent a track
    /// being played twice in a row
    last_track: String,
    /// The previous track's activity kind, for transitions
    last_activity: MusicActivity,
}

#[derive(Deserialize)]
pub struct MusicTransitionManifest {
    /// Within what radius do enemies count towards combat music?
    combat_nearby_radius: f32,
    /// Each multiple of this factor that an enemy has health counts as an extra
    /// enemy
    combat_health_factor: u32,
    /// How many nearby enemies trigger High combat music
    combat_nearby_high_thresh: u32,
    /// How many nearby enemies trigger Low combat music
    combat_nearby_low_thresh: u32,
    /// Fade in and fade out timings for transitions between channels
    pub fade_timings: HashMap<(MusicChannelTag, MusicChannelTag), (f32, f32)>,
}

impl Default for MusicTransitionManifest {
    fn default() -> MusicTransitionManifest {
        MusicTransitionManifest {
            combat_nearby_radius: 40.0,
            combat_health_factor: 1000,
            combat_nearby_high_thresh: 3,
            combat_nearby_low_thresh: 1,
            fade_timings: HashMap::new(),
        }
    }
}

impl assets::Asset for MusicTransitionManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";

    fn default_value(id: &str, e: assets::Error) -> Result<MusicTransitionManifest, assets::Error> {
        warn!("Error loading MusicTransitionManifest {:?}: {:?}", id, e);
        Ok(MusicTransitionManifest::default())
    }
}

lazy_static! {
    pub static ref MUSIC_TRANSITION_MANIFEST: AssetHandle<MusicTransitionManifest> =
        AssetExt::load_expect("voxygen.audio.music_transition_manifest");
}

impl Default for MusicMgr {
    fn default() -> Self {
        Self {
            soundtrack: Self::load_soundtrack_items(),
            began_playing: Instant::now(),
            next_track_change: 0.0,
            last_track: String::from("None"),
            last_activity: MusicActivity::State(MusicActivityState::Explore),
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

        use common::comp::{group::ENEMY, Group, Health, Pos};
        use specs::{Join, WorldExt};
        use MusicActivityState::*;
        let mut activity_state = Explore;
        let player = client.entity();
        let ecs = state.ecs();
        let entities = ecs.entities();
        let positions = ecs.read_component::<Pos>();
        let healths = ecs.read_component::<Health>();
        let groups = ecs.read_component::<Group>();
        let mtm = MUSIC_TRANSITION_MANIFEST.read();
        if let Some(player_pos) = positions.get(player) {
            let num_nearby_entities: u32 = (&entities, &positions, &healths, &groups)
                .join()
                .map(|(entity, pos, health, group)| {
                    if entity != player
                        && group == &ENEMY
                        && (player_pos.0 - pos.0).magnitude_squared()
                            < mtm.combat_nearby_radius.powf(2.0)
                    {
                        (health.maximum() / mtm.combat_health_factor).max(1)
                    } else {
                        0
                    }
                })
                .sum();

            if num_nearby_entities >= mtm.combat_nearby_high_thresh {
                activity_state = Combat(CombatIntensity::High);
            } else if num_nearby_entities >= mtm.combat_nearby_low_thresh {
                activity_state = Combat(CombatIntensity::Low);
            }
            trace!(
                "in audio maintain: {:?} {:?}",
                activity_state,
                num_nearby_entities
            );
        }

        // Override combat music with explore music if the player is dead
        if let Some(health) = healths.get(player) {
            if health.is_dead {
                activity_state = Explore;
            }
        }

        let activity = match self.last_activity {
            MusicActivity::State(prev) if prev != activity_state => {
                MusicActivity::Transition(prev, activity_state)
            },
            MusicActivity::Transition(_, next) => MusicActivity::State(next),
            _ => MusicActivity::State(activity_state),
        };

        let interrupt = matches!(activity, MusicActivity::Transition(_, _));

        if audio.music_enabled()
            && !self.soundtrack.read().tracks.is_empty()
            && (self.began_playing.elapsed().as_secs_f32() > self.next_track_change || interrupt)
        {
            trace!(
                "pre-play_random_track: {:?} {:?}",
                self.last_activity,
                activity
            );
            if let Ok(next_activity) = self.play_random_track(audio, state, client, &activity) {
                self.last_activity = next_activity;
            }
        }
    }

    fn play_random_track(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        client: &Client,
        activity: &MusicActivity,
    ) -> Result<MusicActivity, ()> {
        let mut rng = thread_rng();

        // Adds a bit of randomness between plays
        let silence_between_tracks_seconds: f32 =
            if matches!(activity, MusicActivity::State(MusicActivityState::Explore)) {
                rng.gen_range(60.0..120.0)
            } else {
                0.0
            };

        let is_dark = (state.get_day_period().is_dark()) as bool;
        let current_period_of_day = Self::get_current_day_period(is_dark);
        let current_biome = client.current_biome();
        let current_site = client.current_site();

        // Filter the soundtrack in stages, so that we don't overprune it if there are
        // too many constraints. Returning Err(()) signals that we couldn't find
        // an appropriate track for the current state, and hence the state
        // machine for the activity shouldn't be updated.
        let soundtrack = self.soundtrack.read();
        // First, filter out tracks not matching the timing, site, and biome
        let mut maybe_tracks = soundtrack
            .tracks
            .iter()
            .filter(|track| {
                (match &track.timing {
                    Some(period_of_day) => period_of_day == &current_period_of_day,
                    None => true,
                }) && match &track.site {
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
        // Second, filter out any tracks that don't match the current activity.
        let filtered_tracks: Vec<_> = maybe_tracks
            .iter()
            .filter(|track| &track.activity == activity)
            .cloned()
            .collect();
        trace!(
            "maybe_len: {}, filtered len: {}",
            maybe_tracks.len(),
            filtered_tracks.len()
        );
        if !filtered_tracks.is_empty() {
            maybe_tracks = filtered_tracks;
        } else {
            return Err(());
        }
        // Third, prevent playing the last track if possible (though don't return Err
        // here, since the combat music is intended to loop)
        let filtered_tracks: Vec<_> = maybe_tracks
            .iter()
            .filter(|track| !track.title.eq(&self.last_track))
            .cloned()
            .collect();
        trace!(
            "maybe_len: {}, filtered len: {}",
            maybe_tracks.len(),
            filtered_tracks.len()
        );
        if !filtered_tracks.is_empty() {
            maybe_tracks = filtered_tracks;
        }

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
        debug!(
            "selecting new track for {:?}: {:?}",
            activity, new_maybe_track
        );

        if let Ok(track) = new_maybe_track {
            //println!("Now playing {:?}", track.title);
            self.last_track = String::from(&track.title);
            self.began_playing = Instant::now();
            self.next_track_change = track.length + silence_between_tracks_seconds;

            let tag = if matches!(activity, MusicActivity::State(MusicActivityState::Explore)) {
                MusicChannelTag::Exploration
            } else {
                MusicChannelTag::Combat
            };
            audio.play_music(&track.path, tag);

            if let Some(state) = track.activity_override {
                Ok(MusicActivity::State(state))
            } else {
                Ok(*activity)
            }
        } else {
            Err(())
        }
    }

    fn get_current_day_period(is_dark: bool) -> DayPeriod {
        if is_dark {
            DayPeriod::Night
        } else {
            DayPeriod::Day
        }
    }

    fn load_soundtrack_items() -> AssetHandle<SoundtrackCollection<SoundtrackItem>> {
        // Cannot fail: A default value is always provided
        SoundtrackCollection::load_expect("voxygen.audio.soundtrack")
    }
}
impl assets::Asset for SoundtrackCollection<RawSoundtrackItem> {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl assets::Compound for SoundtrackCollection<SoundtrackItem> {
    fn load<S: assets::source::Source>(
        _: &assets::AssetCache<S>,
        id: &str,
    ) -> Result<Self, assets::Error> {
        let inner = || -> Result<_, assets::Error> {
            let manifest: AssetHandle<assets::Ron<SoundtrackCollection<RawSoundtrackItem>>> =
                AssetExt::load(id)?;
            let mut soundtracks = SoundtrackCollection::default();
            for item in manifest.read().0.tracks.iter().cloned() {
                match item {
                    RawSoundtrackItem::Individual(track) => soundtracks.tracks.push(track),
                    RawSoundtrackItem::Segmented {
                        title,
                        timing,
                        biomes,
                        site,
                        segments,
                    } => {
                        for (path, length, activity, activity_override) in segments.into_iter() {
                            soundtracks.tracks.push(SoundtrackItem {
                                title: title.clone(),
                                path,
                                length,
                                timing: timing.clone(),
                                biomes: biomes.clone(),
                                site,
                                activity,
                                activity_override,
                            });
                        }
                    },
                }
            }
            Ok(soundtracks)
        };
        match inner() {
            Ok(soundtracks) => Ok(soundtracks),
            Err(e) => {
                warn!("Error loading soundtracks: {:?}", e);
                Ok(SoundtrackCollection::default())
            },
        }
    }
}
