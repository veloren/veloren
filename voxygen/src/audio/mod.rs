//! Handles audio device detection and playback of sound effects and music

pub mod channel;
pub mod fader;
pub mod music;
pub mod sfx;
pub mod soundcache;

use channel::{MusicChannel, MusicChannelTag, SfxChannel};
use fader::Fader;
use soundcache::SoundCache;
use tracing::warn;

use common::assets;
use cpal::traits::DeviceTrait;
use rodio::{source::Source, Decoder, Device};
use vek::*;

const FALLOFF: f32 = 0.13;

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

    listener_pos: Vec3<f32>,
    listener_ori: Vec3<f32>,

    listener_ear_left: Vec3<f32>,
    listener_ear_right: Vec3<f32>,
}

impl AudioFrontend {
    /// Construct with given device
    #[allow(clippy::redundant_clone)] // TODO: Pending review in #587
    pub fn new(device: String, max_sfx_channels: usize) -> Self {
        let mut sfx_channels = Vec::with_capacity(max_sfx_channels);
        let audio_device = get_device_raw(&device);

        if let Some(audio_device) = &audio_device {
            for _ in 0..max_sfx_channels {
                sfx_channels.push(SfxChannel::new(&audio_device));
            }
        }

        Self {
            device: device.clone(),
            device_list: list_devices(),
            audio_device,
            sound_cache: SoundCache::default(),
            music_channels: Vec::new(),
            sfx_channels,
            sfx_volume: 1.0,
            music_volume: 1.0,
            listener_pos: Vec3::zero(),
            listener_ori: Vec3::zero(),
            listener_ear_left: Vec3::zero(),
            listener_ear_right: Vec3::zero(),
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
            listener_pos: Vec3::zero(),
            listener_ori: Vec3::zero(),
            listener_ear_left: Vec3::zero(),
            listener_ear_right: Vec3::zero(),
        }
    }

    /// Drop any unused music channels, and update their faders
    pub fn maintain(&mut self, dt: f32) {
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
                    existing_channel.set_fader(Fader::fade_out(2.0, self.music_volume));

                    let mut next_music_channel = MusicChannel::new(&audio_device);

                    next_music_channel.set_fader(Fader::fade_in(12.0, self.music_volume));

                    self.music_channels.push(next_music_channel);
                }
            }
        }

        self.music_channels.last_mut()
    }

    /// Play (once) an sfx file by file path at the give position and volume
    pub fn play_sfx(&mut self, sound: &str, pos: Vec3<f32>, vol: Option<f32>) {
        if self.audio_device.is_some() {
            let calc_pos = ((pos - self.listener_pos) * FALLOFF).into_array();

            let sound = self
                .sound_cache
                .load_sound(sound)
                .amplify(vol.unwrap_or(1.0));

            let left_ear = self.listener_ear_left.into_array();
            let right_ear = self.listener_ear_right.into_array();

            if let Some(channel) = self.get_sfx_channel() {
                channel.set_emitter_position(calc_pos);
                channel.set_left_ear_position(left_ear);
                channel.set_right_ear_position(right_ear);
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

    pub fn set_listener_pos(&mut self, pos: &Vec3<f32>, ori: &Vec3<f32>) {
        self.listener_pos = *pos;
        self.listener_ori = ori.normalized();

        let up = Vec3::new(0.0, 0.0, 1.0);

        let pos_left = up.cross(self.listener_ori).normalized();
        let pos_right = self.listener_ori.cross(up).normalized();

        self.listener_ear_left = pos_left;
        self.listener_ear_right = pos_right;

        for channel in self.sfx_channels.iter_mut() {
            if !channel.is_done() {
                // TODO: Update this to correctly determine the updated relative position of
                // the SFX emitter when the player (listener) moves
                // channel.set_emitter_position(
                //     ((channel.pos - self.listener_pos) * FALLOFF).into_array(),
                // );
                channel.set_left_ear_position(pos_left.into_array());
                channel.set_right_ear_position(pos_right.into_array());
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
