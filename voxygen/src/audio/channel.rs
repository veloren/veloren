//! Distinct audio playback channels for music and sound effects
//!
//! Voxygen's audio system uses a limited number of channels to play multiple
//! sounds simultaneously. Each additional channel used decreases performance
//! in-game, so the amount of channels utilized should be kept to a minimum.
//!
//! When constructing a new [`AudioFrontend`](../struct.AudioFrontend.html), two
//! music channels are created internally (to achieve crossover fades) while the
//! number of sfx channels are determined by the `num_sfx_channels` value
//! defined in the client
//! [`AudioSettings`](../../settings/struct.AudioSettings.html)
//!
//! When the AudioFrontend's
//! [`emit_sfx`](../struct.AudioFrontend.html#method.emit_sfx)
//! methods is called, it attempts to retrieve an SfxChannel for playback. If
//! the channel capacity has been reached and all channels are occupied, a
//! warning is logged, and no sound is played.

use crate::audio::{
    fader::{FadeDirection, Fader},
    Listener,
};
use rodio::{OutputStreamHandle, Sample, Sink, Source, SpatialSink};
use serde::Deserialize;
use std::time::Instant;
use strum::EnumIter;
use tracing::warn;
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
#[derive(PartialEq, Clone, Copy, Hash, Eq, Deserialize)]
pub enum MusicChannelTag {
    TitleMusic,
    Exploration,
    Combat,
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
    pub fn new(stream: &OutputStreamHandle) -> Self {
        let new_sink = Sink::try_new(stream);
        match new_sink {
            Ok(sink) => Self {
                sink,
                tag: MusicChannelTag::TitleMusic,
                state: ChannelState::Stopped,
                fader: Fader::default(),
            },
            Err(_) => {
                warn!("Failed to create a rodio sink. May not play sounds.");
                Self {
                    sink: Sink::new_idle().0,
                    tag: MusicChannelTag::TitleMusic,
                    state: ChannelState::Stopped,
                    fader: Fader::default(),
                }
            },
        }
    }

    /// Play a music track item on this channel. If the channel has an existing
    /// track playing, the new sounds will be appended and played once they
    /// complete. Otherwise it will begin playing immediately.
    pub fn play<S>(&mut self, source: S, tag: MusicChannelTag)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
        <S as Iterator>::Item: std::fmt::Debug,
    {
        self.tag = tag;
        self.sink.append(source);

        self.state = if !self.fader.is_finished() {
            ChannelState::Fading
        } else {
            ChannelState::Playing
        };
    }

    /// Stop whatever is playing on a given music channel
    pub fn stop(&mut self, tag: MusicChannelTag) {
        self.tag = tag;
        self.sink.stop();
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
    pub fn maintain(&mut self, dt: std::time::Duration) {
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

/// AmbientChannelTags are used for non-positional sfx. Currently the only use
/// is for wind.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, EnumIter)]
pub enum AmbientChannelTag {
    Wind,
    Rain,
    Thunder,
    Leaves,
    Cave,
}

/// A AmbientChannel uses a non-positional audio sink designed to play sounds
/// which are always heard at the camera's position.
pub struct AmbientChannel {
    tag: AmbientChannelTag,
    pub multiplier: f32,
    sink: Sink,
    pub began_playing: Instant,
    pub next_track_change: f32,
}

impl AmbientChannel {
    pub fn new(stream: &OutputStreamHandle, tag: AmbientChannelTag, multiplier: f32) -> Self {
        let new_sink = Sink::try_new(stream);
        match new_sink {
            Ok(sink) => Self {
                tag,
                multiplier,
                sink,
                began_playing: Instant::now(),
                next_track_change: 0.0,
            },
            Err(_) => {
                warn!("Failed to create rodio sink. May not play ambient sounds.");
                Self {
                    tag,
                    multiplier,
                    sink: Sink::new_idle().0,
                    began_playing: Instant::now(),
                    next_track_change: 0.0,
                }
            },
        }
    }

    pub fn play<S>(&mut self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
        <S as Iterator>::Item: std::fmt::Debug,
    {
        self.sink.append(source);
    }

    pub fn stop(&mut self) { self.sink.stop(); }

    pub fn set_volume(&mut self, volume: f32) { self.sink.set_volume(volume * self.multiplier); }

    // pub fn get_volume(&mut self) -> f32 { self.sink.volume() }

    pub fn get_tag(&self) -> AmbientChannelTag { self.tag }

    // pub fn set_tag(&mut self, tag: AmbientChannelTag) { self.tag = tag }
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
    pub fn new(stream: &OutputStreamHandle) -> Self {
        Self {
            sink: SpatialSink::try_new(stream, [0.0; 3], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0])
                .unwrap(),
            pos: Vec3::zero(),
        }
    }

    pub fn play<S>(&mut self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
        <S as Iterator>::Item: std::fmt::Debug,
    {
        self.sink.append(source);
    }

    /// Same as SfxChannel::play but with the source passed through
    /// a low pass filter at 300 Hz
    pub fn play_with_low_pass_filter<S>(&mut self, source: S, freq: u32)
    where
        S: Sized + Send + 'static,
        S: Source<Item = f32>,
    {
        let source = source.low_pass(freq);
        self.sink.append(source);
    }

    pub fn set_volume(&mut self, volume: f32) { self.sink.set_volume(volume); }

    pub fn stop(&mut self) { self.sink.stop(); }

    pub fn is_done(&self) -> bool { self.sink.empty() }

    pub fn set_pos(&mut self, pos: Vec3<f32>) { self.pos = pos; }

    pub fn update(&mut self, listener: &Listener) {
        const FALLOFF: f32 = 0.13;

        self.sink
            .set_emitter_position(((self.pos - listener.pos) * FALLOFF).into_array());
        self.sink
            .set_left_ear_position(listener.ear_left_rpos.into_array());
        self.sink
            .set_right_ear_position(listener.ear_right_rpos.into_array());
    }
}

/// An UiChannel uses a non-spatial audio sink, and is designed for short-lived
/// audio which is not spatially controlled, but does not need control over
/// playback or fading/transitions
///
/// See also: [`Rodio::Sink`](https://docs.rs/rodio/0.11.0/rodio/struct.Sink.html)
pub struct UiChannel {
    sink: Sink,
}

impl UiChannel {
    pub fn new(stream: &OutputStreamHandle) -> Self {
        Self {
            sink: Sink::try_new(stream).unwrap(),
        }
    }

    pub fn play<S>(&mut self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
        <S as Iterator>::Item: std::fmt::Debug,
    {
        self.sink.append(source);
    }

    pub fn set_volume(&mut self, volume: f32) { self.sink.set_volume(volume); }

    pub fn stop(&mut self) { self.sink.stop(); }

    pub fn is_done(&self) -> bool { self.sink.empty() }
}
