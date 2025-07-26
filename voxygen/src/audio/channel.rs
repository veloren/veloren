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

use client::EcsEntity;
use kira::{
    Easing, StartTime, Tween,
    clock::ClockTime,
    listener::ListenerId,
    sound::PlaybackState,
    track::{SpatialTrackBuilder, SpatialTrackHandle, TrackBuilder, TrackHandle},
};
use serde::Deserialize;
use std::time::Duration;
use strum::EnumIter;
use tracing::warn;
use vek::*;

use crate::audio;

use super::soundcache::{AnySoundData, AnySoundHandle};

/// We watch the states of nearby entities in order to emit SFX at their
/// position based on their state. This constant limits the radius that we
/// observe to prevent tracking distant entities. It approximates the distance
/// at which the volume of the sfx emitted is too quiet to be meaningful for the
/// player.
pub const SFX_DIST_LIMIT: f32 = 250.0;
pub const SFX_DIST_LIMIT_SQR: f32 = SFX_DIST_LIMIT * SFX_DIST_LIMIT;

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
    track: TrackHandle,
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
    pub fn new(route_to: &mut TrackHandle) -> Result<Self, kira::ResourceLimitReached> {
        let track = route_to.add_sub_track(TrackBuilder::new().volume(audio::to_decibels(0.0)))?;
        Ok(Self {
            tag: MusicChannelTag::TitleMusic,
            track,
            source: None,
            length: 0.0,
            loop_data: (false, LoopPoint::Start, LoopPoint::End),
        })
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

    pub fn play(
        &mut self,
        mut source: AnySoundData,
        now: ClockTime,
        fade_in: Option<f32>,
        delay: Option<f32>,
    ) {
        if let Some(fade_in) = fade_in {
            let fade_in_tween = Tween {
                duration: Duration::from_secs_f32(fade_in),
                ..Default::default()
            };
            source = source.fade_in_tween(fade_in_tween);
        }

        if let Some(delay) = delay {
            source = source.start_time(now + delay as f64);
        }

        match self.track.play(source) {
            Ok(handle) => self.source = Some(handle),
            Err(e) => {
                warn!(?e, "Cannot play music")
            },
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
        self.track
            .set_volume(audio::to_decibels(volume), Tween::default());
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
        self.track.set_volume(audio::to_decibels(volume), tween);
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

    /// Get a mutable reference to the channel's track
    pub fn get_track(&mut self) -> &mut TrackHandle { &mut self.track }

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
    source: Option<AnySoundHandle>,
    pub looping: bool,
}

impl AmbienceChannel {
    pub fn new(
        tag: AmbienceChannelTag,
        init_volume: f32,
        route_to: &mut TrackHandle,
        looping: bool,
    ) -> Result<Self, kira::ResourceLimitReached> {
        let ambience_track_builder = TrackBuilder::new();
        let track =
            route_to.add_sub_track(ambience_track_builder.volume(audio::to_decibels(0.0)))?;

        Ok(Self {
            tag,
            target_volume: init_volume,
            track,
            source: None,
            looping,
        })
    }

    pub fn set_source(&mut self, source_handle: Option<AnySoundHandle>) {
        self.source = source_handle;
    }

    pub fn play(&mut self, mut source: AnySoundData, fade_in: Option<f32>, delay: Option<f32>) {
        let mut tween = Tween::default();
        if let Some(fade_in) = fade_in {
            tween.duration = Duration::from_secs_f32(fade_in);
        }
        if let Some(delay) = delay {
            tween.start_time = StartTime::Delayed(Duration::from_secs_f32(delay));
        }
        source = source.fade_in_tween(tween);
        match self.track.play(source) {
            Ok(handle) => self.source = Some(handle),
            Err(e) => {
                warn!(?e, "Cannot play ambience")
            },
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
        }
    }

    /// Set the channel to a volume, fading over a given duration
    pub fn fade_to(&mut self, volume: f32, duration: f32) {
        self.track.set_volume(audio::to_decibels(volume), Tween {
            start_time: StartTime::Immediate,
            duration: Duration::from_secs_f32(duration),
            easing: Easing::Linear,
        });
        self.target_volume = volume;
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
    track: SpatialTrackHandle,
    source: Option<AnySoundHandle>,
    pub pos: Vec3<f32>,
    // Allow the position to be updated over time
    pub pos_entity: Option<EcsEntity>,
}

impl SfxChannel {
    pub fn new(
        route_to: &mut TrackHandle,
        listener: ListenerId,
    ) -> Result<Self, kira::ResourceLimitReached> {
        let sfx_track_builder = SpatialTrackBuilder::new()
            .distances((1.0, SFX_DIST_LIMIT))
            .attenuation_function(Some(Easing::OutPowf(0.66)));
        let track = route_to.add_spatial_sub_track(listener, Vec3::zero(), sfx_track_builder)?;
        Ok(Self {
            track,
            source: None,
            pos: Vec3::zero(),
            pos_entity: None,
        })
    }

    pub fn set_source(&mut self, source_handle: Option<AnySoundHandle>) {
        self.source = source_handle;
    }

    pub fn play(&mut self, source: AnySoundData) {
        match self.track.play(source) {
            Ok(handle) => self.source = Some(handle),
            Err(e) => {
                warn!(?e, "Cannot play sfx")
            },
        }
    }

    pub fn stop(&mut self) {
        if let Some(source) = self.source.as_mut() {
            source.stop(Tween::default())
        }
    }

    /// Sets volume of the track, not the source. This is to be used only for
    /// multiplying the volume post distance calculation.
    pub fn set_volume(&mut self, volume: f32) {
        let tween = Tween {
            duration: Duration::from_secs_f32(0.0),
            ..Default::default()
        };
        self.track.set_volume(audio::to_decibels(volume), tween)
    }

    pub fn is_done(&self) -> bool {
        self.source
            .as_ref()
            .is_none_or(|source| source.state() == PlaybackState::Stopped)
    }

    /// Update volume of sounds based on position of player
    pub fn update(&mut self, emitter_pos: Vec3<f32>, player_pos: Vec3<f32>) {
        let tween = Tween {
            duration: Duration::from_secs_f32(0.0),
            ..Default::default()
        };
        self.track.set_position(emitter_pos, tween);
        self.pos = emitter_pos;

        // A multiplier between 0.0 and 1.0, with 0.0 being the furthest away from and
        // 1.0 being closest to the player.
        let ratio = 1.0
            - (player_pos.distance(self.pos) / SFX_DIST_LIMIT)
                .clamp(0.0, 1.0)
                .sqrt();
        self.set_volume(ratio);
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum UiChannelTag {
    LevelUp,
}

/// An UiChannel uses a non-spatial audio sink, and is designed for short-lived
/// audio which is not spatially controlled, but does not need control over
/// playback or fading/transitions
pub struct UiChannel {
    track: TrackHandle,
    source: Option<AnySoundHandle>,
    pub tag: Option<UiChannelTag>,
}

impl UiChannel {
    pub fn new(route_to: &mut TrackHandle) -> Result<Self, kira::ResourceLimitReached> {
        let track = route_to.add_sub_track(TrackBuilder::default())?;
        Ok(Self {
            track,
            source: None,
            tag: None,
        })
    }

    pub fn set_source(&mut self, source_handle: Option<AnySoundHandle>) {
        self.source = source_handle;
    }

    pub fn play(&mut self, source: AnySoundData, tag: Option<UiChannelTag>) {
        match self.track.play(source) {
            Ok(handle) => {
                self.source = Some(handle);
                self.tag = tag;
            },
            Err(e) => {
                warn!(?e, "Cannot play ui sfx")
            },
        }
    }

    pub fn stop(&mut self) {
        if let Some(source) = self.source.as_mut() {
            source.stop(Tween::default())
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.track
            .set_volume(audio::to_decibels(volume), Tween::default())
    }

    pub fn is_done(&self) -> bool {
        self.source
            .as_ref()
            .is_none_or(|source| source.state() == PlaybackState::Stopped)
    }
}

#[cfg(test)]
mod tests {
    use crate::audio::channel::{SFX_DIST_LIMIT, SFX_DIST_LIMIT_SQR};

    #[test]
    // Small optimization so sqrt() isn't called at runtime
    fn test_sfx_dist_limit_eq_sfx_dist_limit_sqr() {
        assert!(SFX_DIST_LIMIT.powf(2.0) == SFX_DIST_LIMIT_SQR)
    }
}
