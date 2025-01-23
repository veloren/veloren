//! Distinct audio playback channels for music and sound effects
//!
//! Voxygen's audio system uses a limited number of channels to play multiple
//! sounds simultaneously. Each additional channel used decreases performance
//! in-game, so the amount of channels utilized should be kept to a minimum.
//!
//! When constructing a new [`AudioFrontend`](../struct.AudioFrontend.html), the
//! number of sfx channels are determined by the `num_sfx_channels` value
//! defined in the client
//! [`AudioSettings`](../../settings/struct.AudioSettings.html)

use kira::{
    StartTime, Volume,
    effect::filter::{FilterBuilder, FilterHandle},
    manager::AudioManager,
    sound::PlaybackState,
    spatial::emitter::EmitterHandle,
    track::{TrackBuilder, TrackHandle, TrackId, TrackRoutes},
    tween::{Easing, Tween, Value},
};
use serde::Deserialize;
use std::time::Duration;
use strum::EnumIter;
use tracing::warn;
use vek::*;

use super::soundcache::AnySoundHandle;

/// Each `MusicChannel` has a `MusicChannelTag` which help us determine when we
/// should transition between two types of in-game music. For example, we
/// transition between `TitleMusic` and `Exploration` when a player enters the
/// world by crossfading over a slow duration. In the future, transitions in the
/// world such as `Exploration` -> `BossBattle` would transition more rapidly.
#[derive(PartialEq, Clone, Copy, Hash, Eq, Deserialize, Debug)]
pub enum MusicChannelTag {
    TitleMusic,
    Exploration,
    Combat,
}

/// A MusicChannel is designed to play music which
/// is always heard at the player's position.
pub struct MusicChannel {
    tag: MusicChannelTag,
    track: Option<TrackHandle>,
    source: Option<AnySoundHandle>,
    length: f32,
    loop_data: (bool, LoopPoint, LoopPoint), // Loops?, Start, End
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoopPoint {
    Start,
    End,
    Point(f64),
}

impl MusicChannel {
    pub fn new(manager: &mut AudioManager, parent_track: TrackId) -> Self {
        let new_track = manager.add_sub_track(
            TrackBuilder::new()
                .volume(Volume::Amplitude(0.0))
                .routes(TrackRoutes::parent(parent_track)),
        );
        match new_track {
            Ok(track) => Self {
                tag: MusicChannelTag::TitleMusic,
                track: Some(track),
                source: None,
                length: 0.0,
                loop_data: (false, LoopPoint::Start, LoopPoint::End),
            },
            Err(_) => {
                warn!(?new_track, "Failed to create track. May not play music.");
                Self {
                    tag: MusicChannelTag::TitleMusic,
                    track: None,
                    source: None,
                    length: 0.0,
                    loop_data: (false, LoopPoint::Start, LoopPoint::End),
                }
            },
        }
    }

    pub fn set_tag(&mut self, tag: MusicChannelTag) { self.tag = tag; }

    pub fn set_source(&mut self, source_handle: Option<AnySoundHandle>) {
        self.source = source_handle;
    }

    pub fn set_length(&mut self, length: f32) { self.length = length; }

    // Gets the currently set loop data
    pub fn get_loop_data(&self) -> (bool, LoopPoint, LoopPoint) { self.loop_data }

    /// Sets whether the sound loops, and the start and end points of the loop
    pub fn set_loop_data(&mut self, loops: bool, start: LoopPoint, end: LoopPoint) {
        if let Some(source) = self.source.as_mut() {
            self.loop_data = (loops, start, end);
            if loops {
                match (start, end) {
                    (LoopPoint::Start, LoopPoint::End) => {
                        source.set_loop_region(0.0..);
                    },
                    (LoopPoint::Start, LoopPoint::Point(end)) => {
                        source.set_loop_region(..end);
                    },
                    (LoopPoint::Point(start), LoopPoint::End) => {
                        source.set_loop_region(start..);
                    },
                    (LoopPoint::Point(start), LoopPoint::Point(end)) => {
                        source.set_loop_region(start..end);
                    },
                    _ => {
                        warn!("Invalid loop points given")
                    },
                }
            } else {
                source.set_loop_region(None);
            }
        }
    }

    /// Stop whatever is playing on this channel with an optional fadeout and
    /// delay
    pub fn stop(&mut self, duration: Option<f32>, delay: Option<f32>) {
        if let Some(source) = self.source.as_mut() {
            let tween = Tween {
                duration: Duration::from_secs_f32(duration.unwrap_or(0.1)),
                start_time: StartTime::Delayed(Duration::from_secs_f32(delay.unwrap_or(0.0))),
                ..Default::default()
            };
            source.stop(tween)
        };
    }

    /// Set the volume of the current channel.
    pub fn set_volume(&mut self, volume: f32) {
        if let Some(track) = self.track.as_mut() {
            track.set_volume(Volume::Amplitude(volume as f64), Tween::default());
            // } else {
            //     warn!("Music track not present; cannot set volume")
        }
    }

    /// Fade to a given amplitude over a given duration, optionally after a
    /// delay
    pub fn fade_to(&mut self, volume: f32, duration: f32, delay: Option<f32>) {
        let mut start_time = StartTime::Immediate;
        if let Some(delay) = delay {
            start_time = StartTime::Delayed(Duration::from_secs_f32(delay))
        }
        let tween = Tween {
            start_time,
            duration: Duration::from_secs_f32(duration),
            easing: Easing::Linear,
        };
        if let Some(track) = self.track.as_mut() {
            track.set_volume(Volume::Amplitude(volume as f64), tween);
        }
    }

    /// Fade to silence over a given duration and stop, optionally after a delay
    /// Use fade_to() if this fade is temporary
    pub fn fade_out(&mut self, duration: f32, delay: Option<f32>) {
        self.stop(Some(duration), delay);
    }

    /// Returns true if the sound has stopped playing (whether by fading out or
    /// by finishing)
    pub fn is_done(&self) -> bool {
        self.source
            .as_ref()
            .is_none_or(|source| source.state() == PlaybackState::Stopped)
    }

    pub fn get_tag(&self) -> MusicChannelTag { self.tag }

    /// Get an immutable reference to the channel's track for purposes of
    /// setting the output destination of a sound
    pub fn get_track(&self) -> Option<&TrackHandle> { self.track.as_ref() }

    /// Get a mutable reference to the channel's track
    pub fn get_track_mut(&mut self) -> Option<&mut TrackHandle> { self.track.as_mut() }

    pub fn get_source(&mut self) -> Option<&mut AnySoundHandle> { self.source.as_mut() }

    pub fn get_length(&self) -> f32 { self.length }
}

/// AmbienceChannelTags are used for non-positional sfx. Currently the only use
/// is for wind.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, EnumIter)]
pub enum AmbienceChannelTag {
    Wind,
    Rain,
    ThunderRumbling,
    Leaves,
    Cave,
    Thunder,
}

/// An AmbienceChannel uses a non-positional audio sink designed to play sounds
/// which are always heard at the camera's position.
#[derive(Debug)]
pub struct AmbienceChannel {
    tag: AmbienceChannelTag,
    target_volume: f32,
    track: TrackHandle,
    filter: FilterHandle,
    source: Option<AnySoundHandle>,
    pub looping: bool,
}

impl AmbienceChannel {
    pub fn new(
        tag: AmbienceChannelTag,
        init_volume: f32,
        manager: &mut AudioManager,
        parent_track: TrackId,
        looping: bool,
    ) -> Result<Self, kira::ResourceLimitReached> {
        let ambience_filter_builder = FilterBuilder::new().cutoff(Value::Fixed(20000.0));
        let mut ambience_track_builder = TrackBuilder::new();
        let filter = ambience_track_builder.add_effect(ambience_filter_builder);
        let track = manager.add_sub_track(
            ambience_track_builder
                .volume(0.0)
                .routes(TrackRoutes::parent(parent_track)),
        )?;

        Ok(Self {
            tag,
            target_volume: init_volume,
            track,
            filter,
            source: None,
            looping,
        })
    }

    pub fn set_source(&mut self, source_handle: Option<AnySoundHandle>) {
        self.source = source_handle;
    }

    /// Stop whatever is playing on this channel with an optional fadeout and
    /// delay
    pub fn stop(&mut self, duration: Option<f32>, delay: Option<f32>) {
        if let Some(source) = self.source.as_mut() {
            let tween = Tween {
                duration: Duration::from_secs_f32(duration.unwrap_or(0.1)),
                start_time: StartTime::Delayed(Duration::from_secs_f32(delay.unwrap_or(0.0))),
                ..Default::default()
            };
            source.stop(tween)
        }
    }

    /// Set the channel to a volume, fading over a given duration
    pub fn fade_to(&mut self, volume: f32, duration: f32) {
        self.track
            .set_volume(Volume::Amplitude(volume as f64), Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_secs_f32(duration),
                easing: Easing::Linear,
            });
        self.target_volume = volume;
    }

    /// Set the cutoff for the lowpass filter on this channel
    pub fn set_filter(&mut self, frequency: u32) {
        self.filter
            .set_cutoff(Value::Fixed(frequency as f64), Tween::default());
    }

    /// Set whether this channel's sound loops or not
    pub fn set_looping(&mut self, loops: bool) {
        if let Some(source) = self.source.as_mut() {
            if loops {
                source.set_loop_region(0.0..);
            } else {
                source.set_loop_region(None);
            }
        }
    }

    pub fn get_source(&mut self) -> Option<&mut AnySoundHandle> { self.source.as_mut() }

    /// Get an immutable reference to the channel's track for purposes of
    /// setting the output destination of a sound
    pub fn get_track(&self) -> &TrackHandle { &self.track }

    /// Get a mutable reference to the channel's track
    pub fn get_track_mut(&mut self) -> &mut TrackHandle { &mut self.track }

    /// Get the volume of this channel. The volume may be in the process of
    /// being faded to.
    pub fn get_target_volume(&self) -> f32 { self.target_volume }

    pub fn get_tag(&self) -> AmbienceChannelTag { self.tag }

    pub fn set_tag(&mut self, tag: AmbienceChannelTag) { self.tag = tag }

    pub fn is_active(&self) -> bool { self.get_target_volume() == 0.0 }

    pub fn is_stopped(&self) -> bool {
        if let Some(source) = self.source.as_ref() {
            source.state() == PlaybackState::Stopped
        } else {
            false
        }
    }
}

/// An SfxChannel uses a positional audio sink, and is designed for short-lived
/// audio which can be spatially controlled, but does not need control over
/// playback or fading/transitions
///
/// Note: currently, emitters are static once spawned
#[derive(Debug)]
pub struct SfxChannel {
    source: Option<AnySoundHandle>,
    emitter: EmitterHandle,
    pub pos: Vec3<f32>,
}

impl SfxChannel {
    pub fn new(emitter: EmitterHandle) -> Self {
        Self {
            source: None,
            emitter,
            pos: Vec3::zero(),
        }
    }

    pub fn set_source(&mut self, source_handle: Option<AnySoundHandle>) {
        self.source = source_handle;
    }

    pub fn stop(&mut self) {
        if let Some(source) = self.source.as_mut() {
            source.stop(Tween::default())
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        if let Some(source) = self.source.as_mut() {
            source.set_volume(Volume::Amplitude(volume as f64), Tween::default())
        }
    }

    pub fn get_emitter(&self) -> &EmitterHandle { &self.emitter }

    pub fn is_done(&self) -> bool {
        self.source
            .as_ref()
            .is_none_or(|source| source.state() == PlaybackState::Stopped)
    }

    pub fn update(&mut self, pos: Vec3<f32>) {
        self.pos = pos;

        self.emitter.set_position(pos, Tween {
            duration: Duration::from_secs_f32(0.0),
            ..Default::default()
        });
    }
}

/// An UiChannel uses a non-spatial audio sink, and is designed for short-lived
/// audio which is not spatially controlled, but does not need control over
/// playback or fading/transitions
pub struct UiChannel {
    track: Option<TrackHandle>,
    source: Option<AnySoundHandle>,
}

impl UiChannel {
    pub fn new(manager: &mut AudioManager, parent_track: TrackId) -> Self {
        let new_track = manager
            .add_sub_track(TrackBuilder::default().routes(TrackRoutes::parent(parent_track)));
        match new_track {
            Ok(track) => Self {
                track: Some(track),
                source: None,
            },
            Err(_) => {
                warn!(
                    ?new_track,
                    "Failed to create track. May not play UI sounds."
                );
                Self {
                    track: None,
                    source: None,
                }
            },
        }
    }

    pub fn set_source(&mut self, source_handle: Option<AnySoundHandle>) {
        self.source = source_handle;
    }

    pub fn stop(&mut self) {
        if let Some(source) = self.source.as_mut() {
            source.stop(Tween::default())
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        if let Some(track) = self.track.as_mut() {
            track.set_volume(Volume::Amplitude(volume as f64), Tween::default())
            // } else {
            //     warn!("UI track not present; cannot set volume")
        }
    }

    pub fn is_done(&self) -> bool {
        self.source
            .as_ref()
            .is_none_or(|source| source.state() == PlaybackState::Stopped)
    }
}
