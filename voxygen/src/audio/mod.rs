//! Handles audio device detection and playback of sound effects and music

pub mod channel;
pub mod fader;
pub mod music;
pub mod sfx;
pub mod soundcache;

use channel::{MusicChannel, MusicChannelTag, SfxChannel};
use fader::Fader;
use soundcache::SoundCache;
use std::time::Duration;
use tracing::warn;

use common::assets;
use cpal::traits::DeviceTrait;
use rodio::{source::Source, Decoder, Device};
use vek::*;

#[derive(Default, Clone)]
pub struct Listener {
    pos: Vec3<f32>,
    ori: Vec3<f32>,

    ear_left_rpos: Vec3<f32>,
    ear_right_rpos: Vec3<f32>,
}

/// Holds information about the system audio devices and internal channels used
/// for sfx and music playback. An instance of `AudioFrontend` is used by
/// Voxygen's [`GlobalState`](../struct.GlobalState.html#structfield.audio) to
/// provide access to devices and playback control in-game
pub struct AudioFrontend {
    pub device: String,
    pub device_list: Vec<String>,
    audio_device: Option<Device>,
    sound_cache: SoundCache,

    music_channels: Vec<MusicChannel>,
    sfx_channels: Vec<SfxChannel>,
    sfx_volume: f32,
    music_volume: f32,
    listener: Listener,
}

impl AudioFrontend {
    /// Construct with given device
    pub fn new(device: String, max_sfx_channels: usize) -> Self {
        let audio_device = get_device_raw(&device);

        let mut sfx_channels = Vec::with_capacity(max_sfx_channels);
        if let Some(audio_device) = &audio_device {
            sfx_channels.resize_with(max_sfx_channels, || SfxChannel::new(&audio_device));
        }

        Self {
            device,
            device_list: list_devices(),
            audio_device,
            sound_cache: SoundCache::default(),
            music_channels: Vec::new(),
            sfx_channels,
            sfx_volume: 1.0,
            music_volume: 1.0,
            listener: Listener::default(),
        }
    }

    /// Construct in `no-audio` mode for debugging
    pub fn no_audio() -> Self {
        Self {
            device: "none".to_string(),
            device_list: Vec::new(),
            audio_device: None,
            sound_cache: SoundCache::default(),
            music_channels: Vec::new(),
            sfx_channels: Vec::new(),
            sfx_volume: 1.0,
            music_volume: 1.0,
            listener: Listener::default(),
        }
    }

    /// Drop any unused music channels, and update their faders
    pub fn maintain(&mut self, dt: Duration) {
        self.music_channels.retain(|c| !c.is_done());

        for channel in self.music_channels.iter_mut() {
            channel.maintain(dt);
        }
    }

    fn get_sfx_channel(&mut self) -> Option<&mut SfxChannel> {
        if self.audio_device.is_some() {
            if let Some(channel) = self.sfx_channels.iter_mut().find(|c| c.is_done()) {
                channel.set_volume(self.sfx_volume);

                return Some(channel);
            }
        }

        None
    }

    /// Retrieve a music channel from the channel list. This inspects the
    /// MusicChannelTag to determine whether we are transitioning between
    /// music types and acts accordingly. For example transitioning between
    /// `TitleMusic` and `Exploration` should fade out the title channel and
    /// fade in a new `Exploration` channel.
    fn get_music_channel(
        &mut self,
        next_channel_tag: MusicChannelTag,
    ) -> Option<&mut MusicChannel> {
        if let Some(audio_device) = &self.audio_device {
            if self.music_channels.is_empty() {
                let mut next_music_channel = MusicChannel::new(&audio_device);
                next_music_channel.set_volume(self.music_volume);

                self.music_channels.push(next_music_channel);
            } else {
                let existing_channel = self.music_channels.last_mut()?;

                if existing_channel.get_tag() != next_channel_tag {
                    // Fade the existing channel out. It will be removed when the fade completes.
                    existing_channel
                        .set_fader(Fader::fade_out(Duration::from_secs(2), self.music_volume));

                    let mut next_music_channel = MusicChannel::new(&audio_device);

                    next_music_channel
                        .set_fader(Fader::fade_in(Duration::from_secs(12), self.music_volume));

                    self.music_channels.push(next_music_channel);
                }
            }
        }

        self.music_channels.last_mut()
    }

    /// Play (once) an sfx file by file path at the give position and volume
    pub fn play_sfx(&mut self, sound: &str, pos: Vec3<f32>, vol: Option<f32>) {
        if self.audio_device.is_some() {
            let sound = self
                .sound_cache
                .load_sound(sound)
                .amplify(vol.unwrap_or(1.0));

            let listener = self.listener.clone();
            if let Some(channel) = self.get_sfx_channel() {
                channel.set_pos(pos);
                channel.update(&listener);
                channel.play(sound);
            }
        }
    }

    fn play_music(&mut self, sound: &str, channel_tag: MusicChannelTag) {
        if let Some(channel) = self.get_music_channel(channel_tag) {
            let file = assets::load_file(&sound, &["ogg"]).expect("Failed to load sound");
            let sound = Decoder::new(file).expect("Failed to decode sound");

            channel.play(sound, channel_tag);
        }
    }

    fn fade_out_music(&mut self, channel_tag: MusicChannelTag) {
        let music_volume = self.music_volume;
        if let Some(channel) = self.get_music_channel(channel_tag) {
            channel.set_fader(Fader::fade_out(5.0, music_volume));
        }
    }

    fn fade_in_music(&mut self, channel_tag: MusicChannelTag) {
        let music_volume = self.music_volume;
        if let Some(channel) = self.get_music_channel(channel_tag) {
            channel.set_fader(Fader::fade_in(5.0, music_volume));
        }
    }

    fn stop_music(&mut self, channel_tag: MusicChannelTag) {
        if let Some(channel) = self.get_music_channel(channel_tag) {
            channel.stop(channel_tag);
        }
    }

    pub fn set_listener_pos(&mut self, pos: Vec3<f32>, ori: Vec3<f32>) {
        self.listener.pos = pos;
        self.listener.ori = ori.normalized();

        let up = Vec3::new(0.0, 0.0, 1.0);
        self.listener.ear_left_rpos = up.cross(self.listener.ori).normalized();
        self.listener.ear_right_rpos = -up.cross(self.listener.ori).normalized();

        for channel in self.sfx_channels.iter_mut() {
            if !channel.is_done() {
                channel.update(&self.listener);
            }
        }
    }

    /// Switches the playing music to the title music, which is pinned to a
    /// specific sound file (veloren_title_tune.ogg)
    pub fn play_title_music(&mut self) {
        if self.music_enabled() {
            self.play_music(
                "voxygen.audio.soundtrack.veloren_title_tune",
                MusicChannelTag::TitleMusic,
            )
        }
    }

    pub fn play_exploration_music(&mut self, item: &str) {
        if self.music_enabled() {
            self.play_music(item, MusicChannelTag::Exploration)
        }
    }

    pub fn fade_out_exploration_music(&mut self) {
        if self.music_enabled() {
            self.fade_out_music(MusicChannelTag::Exploration)
        }
    }

    pub fn fade_in_exploration_music(&mut self) {
        if self.music_enabled() {
            self.fade_in_music(MusicChannelTag::Exploration)
        }
    }

    pub fn stop_exploration_music(&mut self) {
        if self.music_enabled() {
            self.stop_music(MusicChannelTag::Exploration)
        }
    }

    pub fn get_sfx_volume(&self) -> f32 { self.sfx_volume }

    pub fn get_music_volume(&self) -> f32 { self.music_volume }

    pub fn sfx_enabled(&self) -> bool { self.sfx_volume > 0.0 }

    pub fn music_enabled(&self) -> bool { self.music_volume > 0.0 }

    pub fn set_sfx_volume(&mut self, sfx_volume: f32) {
        self.sfx_volume = sfx_volume;

        for channel in self.sfx_channels.iter_mut() {
            channel.set_volume(sfx_volume);
        }
    }

    pub fn set_music_volume(&mut self, music_volume: f32) {
        self.music_volume = music_volume;

        for channel in self.music_channels.iter_mut() {
            channel.set_volume(music_volume);
        }
    }

    // TODO: figure out how badly this will break things when it is called
    pub fn set_device(&mut self, name: String) {
        self.device = name.clone();
        self.audio_device = get_device_raw(&name);
    }
}

/// Returns the default audio device.
/// Does not return rodio Device struct in case our audio backend changes.
pub fn get_default_device() -> Option<String> {
    match rodio::default_output_device() {
        Some(x) => Some(x.name().ok()?),
        None => None,
    }
}

/// Returns a vec of the audio devices available.
/// Does not return rodio Device struct in case our audio backend changes.
pub fn list_devices() -> Vec<String> {
    list_devices_raw()
        .iter()
        .map(|x| x.name().unwrap())
        .collect()
}

/// Returns vec of devices
fn list_devices_raw() -> Vec<Device> {
    match rodio::output_devices() {
        Ok(devices) => {
            // Filter out any devices that the name isn't available for
            devices.filter(|d| d.name().is_ok()).collect()
        },
        Err(_) => {
            warn!("Failed to enumerate audio output devices, audio will not be available");
            Vec::new()
        },
    }
}

fn get_device_raw(device: &str) -> Option<Device> {
    list_devices_raw()
        .into_iter()
        .find(|d| d.name().unwrap() == device)
}
