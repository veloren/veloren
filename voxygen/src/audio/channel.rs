//! Distinct audio playback channels for music and sound effects
//!
//! Voxygen's audio system uses a limited number of channels to play multiple
//! sounds simultaneously. Each additional channel used decreases performance
//! in-game, so the amount of channels utilized should be kept to a minimum.
//!
//! When constructing a new [`AudioFrontend`](../struct.AudioFrontend.html), two
//! music channels are created internally (to achieve crossover fades) while the
//! number of sfx channels are determined by the `max_sfx_channels` value
//! defined in the client
//! [`AudioSettings`](../../settings/struct.AudioSettings.html)
//!
//! When the AudioFrontend's
//! [`play_sfx`](../struct.AudioFrontend.html#method.play_sfx)
//! methods is called, it attempts to retrieve an SfxChannel for playback. If
//! the channel capacity has been reached and all channels are occupied, a
//! warning is logged, and no sound is played.

use crate::audio::fader::{FadeDirection, Fader};
use rodio::{Device, Sample, Sink, Source, SpatialSink};
use vek::*;

#[derive(PartialEq, Clone, Copy)]
enum ChannelState {
    Playing,
    Fading,
    Stopped,
}

/// Each `MusicChannel` has a `MusicChannelTag` which help us determine when we
/// should transition between two types of in-game music. For example, we
/// transition between `TitleMusic` and `Exploration` when a player enters the
/// world by crossfading over a slow duration. In the future, transitions in the
/// world such as `Exploration` -> `BossBattle` would transition more rapidly.
#[derive(PartialEq, Clone, Copy)]
pub enum MusicChannelTag {
    TitleMusic,
    Exploration,
}

/// A MusicChannel uses a non-positional audio sink designed to play music which
/// is always heard at the player's position.
///
/// See also: [`Rodio::Sink`](https://docs.rs/rodio/0.11.0/rodio/struct.Sink.html)
pub struct MusicChannel {
    tag: MusicChannelTag,
    sink: Sink,
    state: ChannelState,
    fader: Fader,
}

impl MusicChannel {
    pub fn new(device: &Device) -> Self {
        Self {
            sink: Sink::new(device),
            tag: MusicChannelTag::TitleMusic,
            state: ChannelState::Stopped,
            fader: Fader::default(),
        }
    }

    // Play a music track item on this channel. If the channel has an existing track
    // playing, the new sounds will be appended and played once they complete.
    // Otherwise it will begin playing immediately.
    pub fn play<S>(&mut self, source: S, tag: MusicChannelTag)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
        <S as std::iter::Iterator>::Item: std::fmt::Debug,
    {
        self.tag = tag;
        self.sink.append(source);

        self.state = if !self.fader.is_finished() {
            ChannelState::Fading
        } else {
            ChannelState::Playing
        };
    }

    /// Set the volume of the current channel. If the channel is currently
    /// fading, the volume of the fader is updated to this value.
    pub fn set_volume(&mut self, volume: f32) {
        if !self.fader.is_finished() {
            self.fader.update_target_volume(volume);
        } else {
            self.sink.set_volume(volume);
        }
    }

    /// Set a fader for the channel. If a fader exists already, it is replaced.
    /// If the channel has not begun playing, and the fader is set to fade in,
    /// we set the volume of the channel to the initial volume of the fader so
    /// that the volumes match when playing begins.
    pub fn set_fader(&mut self, fader: Fader) {
        self.fader = fader;
        self.state = ChannelState::Fading;

        if self.state == ChannelState::Stopped && fader.direction() == FadeDirection::In {
            self.sink.set_volume(fader.get_volume());
        }
    }

    /// Returns true if either the channels sink reports itself as empty (no
    /// more sounds in the queue) or we have forcibly set the channels state to
    /// the 'Stopped' state
    pub fn is_done(&self) -> bool { self.sink.empty() || self.state == ChannelState::Stopped }

    pub fn get_tag(&self) -> MusicChannelTag { self.tag }

    /// Maintain the fader attached to this channel. If the channel is not
    /// fading, no action is taken.
    pub fn maintain(&mut self, dt: f32) {
        if self.state == ChannelState::Fading {
            self.fader.update(dt);
            self.sink.set_volume(self.fader.get_volume());

            if self.fader.is_finished() {
                match self.fader.direction() {
                    FadeDirection::Out => {
                        self.state = ChannelState::Stopped;
                        self.sink.stop();
                    },
                    FadeDirection::In => {
                        self.state = ChannelState::Playing;
                    },
                }
            }
        }
    }
}

/// An SfxChannel uses a positional audio sink, and is designed for short-lived
/// audio which can be spatially controlled, but does not need control over
/// playback or fading/transitions
///
/// See also: [`Rodio::SpatialSink`](https://docs.rs/rodio/0.11.0/rodio/struct.SpatialSink.html)
pub struct SfxChannel {
    sink: SpatialSink,
    pub pos: Vec3<f32>,
}

impl SfxChannel {
    pub fn new(device: &Device) -> Self {
        Self {
            sink: SpatialSink::new(device, [0.0; 3], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
            pos: Vec3::zero(),
        }
    }

    pub fn play<S>(&mut self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
        <S as std::iter::Iterator>::Item: std::fmt::Debug,
    {
        self.sink.append(source);
    }

    pub fn set_volume(&mut self, volume: f32) { self.sink.set_volume(volume); }

    pub fn is_done(&self) -> bool { self.sink.empty() }

    pub fn set_emitter_position(&mut self, pos: [f32; 3]) { self.sink.set_emitter_position(pos); }

    pub fn set_left_ear_position(&mut self, pos: [f32; 3]) { self.sink.set_left_ear_position(pos); }

    pub fn set_right_ear_position(&mut self, pos: [f32; 3]) {
        self.sink.set_right_ear_position(pos);
    }
}
