//! Handles audio device detection and playback of sound effects and music

pub mod ambient;
pub mod channel;
pub mod fader;
pub mod music;
pub mod sfx;
pub mod soundcache;

use channel::{AmbientChannel, AmbientChannelTag, MusicChannel, MusicChannelTag, SfxChannel};
use fader::Fader;
use music::MUSIC_TRANSITION_MANIFEST;
use sfx::{SfxEvent, SfxTriggerItem};
use soundcache::{OggSound, WavSound};
use std::time::Duration;
use tracing::{debug, error};

use common::assets::AssetExt;
use rodio::{source::Source, OutputStream, OutputStreamHandle, StreamError};
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
    // The following is for the disabled device switcher
    //pub device: String,
    //pub device_list: Vec<String>,
    //pub audio_device: Option<Device>,
    pub stream: Option<rodio::OutputStream>,
    audio_stream: Option<rodio::OutputStreamHandle>,

    music_channels: Vec<MusicChannel>,
    ambient_channels: Vec<AmbientChannel>,
    sfx_channels: Vec<SfxChannel>,
    sfx_volume: f32,
    music_volume: f32,
    listener: Listener,
}

impl AudioFrontend {
    /// Construct with given device
    pub fn new(/* dev: String, */ max_sfx_channels: usize) -> Self {
        // Commented out until audio device switcher works
        //let audio_device = get_device_raw(&dev);

        //let device = match get_default_device() {
        //    Some(d) => d,
        //    None => "".to_string(),
        //};

        let (stream, audio_stream) = match get_default_stream() {
            Ok(s) => (Some(s.0), Some(s.1)),
            Err(e) => {
                #[cfg(unix)]
                error!(
                    ?e,
                    "failed to construct audio frontend. Is `pulseaudio-alsa` installed?"
                );
                #[cfg(not(unix))]
                error!(?e, "failed to construct audio frontend.");
                (None, None)
            },
        };

        let mut sfx_channels = Vec::with_capacity(max_sfx_channels);
        if let Some(audio_stream) = &audio_stream {
            sfx_channels.resize_with(max_sfx_channels, || SfxChannel::new(audio_stream));
        };

        Self {
            // The following is for the disabled device switcher
            //device,
            //device_list: list_devices(),
            //audio_device,
            stream,
            audio_stream,
            music_channels: Vec::new(),
            sfx_channels,
            ambient_channels: Vec::new(),
            sfx_volume: 1.0,
            music_volume: 1.0,
            listener: Listener::default(),
        }
    }

    /// Construct in `no-audio` mode for debugging
    pub fn no_audio() -> Self {
        Self {
            // The following is for the disabled device switcher
            //device: "".to_string(),
            //device_list: Vec::new(),
            //audio_device: None,
            stream: None,
            audio_stream: None,
            music_channels: Vec::new(),
            sfx_channels: Vec::new(),
            ambient_channels: Vec::new(),
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

    /// Retrive an empty sfx channel from the list
    fn get_sfx_channel(&mut self) -> Option<&mut SfxChannel> {
        if self.audio_stream.is_some() {
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
        if let Some(audio_stream) = &self.audio_stream {
            if self.music_channels.is_empty() {
                let mut next_music_channel = MusicChannel::new(audio_stream);
                next_music_channel.set_volume(self.music_volume);

                self.music_channels.push(next_music_channel);
            } else {
                let existing_channel = self.music_channels.last_mut()?;

                if existing_channel.get_tag() != next_channel_tag {
                    let mtm = MUSIC_TRANSITION_MANIFEST.read();
                    let (fade_out, fade_in) = mtm
                        .fade_timings
                        .get(&(existing_channel.get_tag(), next_channel_tag))
                        .unwrap_or(&(1.0, 1.0));
                    let fade_out = Duration::from_millis((1000.0 * fade_out) as _);
                    let fade_in = Duration::from_millis((1000.0 * fade_in) as _);
                    // Fade the existing channel out. It will be removed when the fade completes.
                    existing_channel.set_fader(Fader::fade_out(fade_out, self.music_volume));

                    let mut next_music_channel = MusicChannel::new(&audio_stream);

                    next_music_channel.set_fader(Fader::fade_in(fade_in, self.music_volume));

                    self.music_channels.push(next_music_channel);
                }
            }
        }

        self.music_channels.last_mut()
    }

    /// Function to play sfx from external places. Useful for UI and
    /// inventory events
    pub fn emit_sfx_item(&mut self, trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>) {
        if let Some((event, item)) = trigger_item {
            let sfx_file = match item.files.len() {
                0 => {
                    debug!("Sfx event {:?} is missing audio file.", event);
                    "voxygen.audio.sfx.placeholder"
                },
                1 => item
                    .files
                    .last()
                    .expect("Failed to determine sound file for this trigger item."),
                _ => {
                    // If more than one file is listed, choose one at random
                    let rand_step = rand::random::<usize>() % item.files.len();
                    &item.files[rand_step]
                },
            };

            self.play_sfx(sfx_file, self.listener.pos, None);
        } else {
            debug!("Missing sfx trigger config for external sfx event.",);
        }
    }

    /// Play an sfx file given the position, SfxEvent, and whether it is
    /// underwater or not
    pub fn emit_sfx(
        &mut self,
        trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
        position: Vec3<f32>,
        volume: Option<f32>,
        underwater: bool,
    ) {
        if let Some((event, item)) = trigger_item {
            let sfx_file = match item.files.len() {
                0 => {
                    debug!("Sfx event {:?} is missing audio file.", event);
                    "voxygen.audio.sfx.placeholder"
                },
                1 => item
                    .files
                    .last()
                    .expect("Failed to determine sound file for this trigger item."),
                _ => {
                    // If more than one file is listed, choose one at random
                    let rand_step = rand::random::<usize>() % item.files.len();
                    &item.files[rand_step]
                },
            };

            if underwater {
                self.play_underwater_sfx(sfx_file, position, volume);
            } else {
                self.play_sfx(sfx_file, position, volume);
            }
        } else {
            debug!(
                "Missing sfx trigger config for sfx event at position: {:?}",
                position
            );
        }
    }

    /// Play (once) an sfx file by file path at the give position and volume
    pub fn play_sfx(&mut self, sound: &str, pos: Vec3<f32>, vol: Option<f32>) {
        if self.audio_stream.is_some() {
            let sound = WavSound::load_expect(sound)
                .cloned()
                .decoder()
                .amplify(vol.unwrap_or(1.0));

            let listener = self.listener.clone();
            if let Some(channel) = self.get_sfx_channel() {
                channel.set_pos(pos);
                channel.update(&listener);
                channel.play(sound);
            }
        }
    }

    /// Play (once) an sfx file by file path at the give position and volume
    /// but with the sound passed through a low pass filter to simulate
    /// being underwater
    pub fn play_underwater_sfx(&mut self, sound: &str, pos: Vec3<f32>, vol: Option<f32>) {
        if self.audio_stream.is_some() {
            let sound = WavSound::load_expect(sound)
                .cloned()
                .decoder()
                .amplify(vol.unwrap_or(1.0));

            let listener = self.listener.clone();
            if let Some(channel) = self.get_sfx_channel() {
                channel.set_pos(pos);
                channel.update(&listener);
                channel.play_with_low_pass_filter(sound.convert_samples());
            }
        }
    }

    fn play_ambient(
        &mut self,
        channel_tag: AmbientChannelTag,
        sound: &str,
        volume_multiplier: f32,
    ) {
        if self.audio_stream.is_some() {
            if let Some(channel) = self.get_ambient_channel(channel_tag, volume_multiplier) {
                let sound = OggSound::load_expect(sound).cloned().decoder();
                channel.play(sound);
            }
        }
    }

    fn get_ambient_channel(
        &mut self,
        channel_tag: AmbientChannelTag,
        volume_multiplier: f32,
    ) -> Option<&mut AmbientChannel> {
        if let Some(audio_stream) = &self.audio_stream {
            if self.ambient_channels.is_empty() {
                let mut ambient_channel = AmbientChannel::new(audio_stream, channel_tag);
                ambient_channel.set_volume(self.sfx_volume * volume_multiplier);
                self.ambient_channels.push(ambient_channel);
            } else {
                for channel in self.ambient_channels.iter_mut() {
                    if channel.get_tag() == channel_tag {
                        channel.set_volume(self.sfx_volume * volume_multiplier);
                        return Some(channel);
                    }
                }
            }
        }

        None
    }

    fn set_ambient_volume(&mut self, volume_multiplier: f32) {
        if self.audio_stream.is_some() {
            if let Some(channel) = self.ambient_channels.iter_mut().last() {
                channel.set_volume(self.sfx_volume * volume_multiplier);
            }
        }
    }

    fn get_ambient_volume(&mut self) -> f32 {
        if self.audio_stream.is_some() {
            if let Some(channel) = self.ambient_channels.iter_mut().last() {
                channel.get_volume() / self.sfx_volume
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    fn play_music(&mut self, sound: &str, channel_tag: MusicChannelTag) {
        if self.music_enabled() {
            if let Some(channel) = self.get_music_channel(channel_tag) {
                let sound = OggSound::load_expect(sound).cloned().decoder();
                channel.play(sound, channel_tag);
            }
        }
    }

    /* These functions are saved for if we want music playback control at some
     * point. They are not used currently but may be useful for later work.
     *
    fn fade_out_music(&mut self, channel_tag: MusicChannelTag) {
        let music_volume = self.music_volume;
        if let Some(channel) = self.get_music_channel(channel_tag) {
            channel.set_fader(Fader::fade_out(Duration::from_secs(5), music_volume));
        }
    }

    fn fade_in_music(&mut self, channel_tag: MusicChannelTag) {
        let music_volume = self.music_volume;
        if let Some(channel) = self.get_music_channel(channel_tag) {
            channel.set_fader(Fader::fade_in(Duration::from_secs(5), music_volume));
        }
    }

    fn stop_music(&mut self, channel_tag: MusicChannelTag) {
        if let Some(channel) = self.get_music_channel(channel_tag) {
            channel.stop(channel_tag);
        }
    }
    */

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

    // The following is for the disabled device switcher
    //// TODO: figure out how badly this will break things when it is called
    //pub fn set_device(&mut self, name: String) {
    //    self.device = name.clone();
    //    self.audio_device = get_device_raw(&name);
    //}
}

// The following is for the disabled device switcher
///// Returns the default audio device.
///// Does not return rodio Device struct in case our audio backend changes.
//pub fn get_default_device() -> Option<String> {
//    match cpal::default_host().default_output_device() {
//        Some(x) => Some(x.name().ok()?),
//        None => None,
//    }
//}

/// Returns the default stream
fn get_default_stream() -> Result<(OutputStream, OutputStreamHandle), StreamError> {
    rodio::OutputStream::try_default()
}

// The following is for the disabled device switcher
///// Returns a stream on the specified device
//pub fn get_stream(
//    device: &rodio::Device,
//) -> Result<(OutputStream, OutputStreamHandle), StreamError> {
//    rodio::OutputStream::try_from_device(device)
//}
//
//fn list_devices_raw() -> Vec<cpal::Device> {
//    match cpal::default_host().devices() {
//        Ok(devices) => devices.filter(|d| d.name().is_ok()).collect(),
//        Err(_) => {
//            warn!("Failed to enumerate audio output devices, audio will not be
// available");            Vec::new()
//        },
//    }
//}
//
///// Returns a vec of the audio devices available.
///// Does not return rodio Device struct in case our audio backend changes.
//fn list_devices() -> Vec<String> {
//    list_devices_raw()
//        .iter()
//        .map(|x| x.name().unwrap())
//        .collect()
//}
//
//fn get_device_raw(device: &str) -> Option<Device> {
//    list_devices_raw()
//        .into_iter()
//        .find(|d| d.name().unwrap() == device)
//}
