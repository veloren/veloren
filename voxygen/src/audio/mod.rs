//! Handles audio device detection and playback of sound effects and music

pub mod ambience;
pub mod channel;
pub mod music;
pub mod sfx;
pub mod soundcache;

use anim::vek::Quaternion;
use channel::{
    AmbienceChannel, AmbienceChannelTag, LoopPoint, MusicChannel, MusicChannelTag, SfxChannel,
    UiChannel,
};
use client::EcsEntity;
use cpal::{
    Device, SampleRate, StreamConfig, SupportedStreamConfigRange,
    traits::{DeviceTrait, HostTrait},
};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, Tween, Value,
    backend::{
        self,
        cpal::{CpalBackend, CpalBackendSettings},
    },
    clock::{ClockHandle, ClockSpeed, ClockTime},
    effect::filter::{FilterBuilder, FilterHandle},
    listener::ListenerHandle,
    track::{TrackBuilder, TrackHandle},
};
use music::MusicTransitionManifest;
use sfx::{SfxEvent, SfxTriggerItem};
use soundcache::load_ogg;
use std::{collections::VecDeque, time::Duration};
use strum::Display;
use tracing::{debug, error, info, warn};

use common::{
    assets::{AssetExt, AssetHandle},
    comp::Ori,
};
use vek::*;

use crate::hud::Subtitle;

// #[derive(Clone)]
// pub struct Listener {
//     pub pos: Vec3<f32>,
//     pub ori: Vec3<f32>,

//     ear_left_rpos: Vec3<f32>,
//     ear_right_rpos: Vec3<f32>,
// }

// impl Default for Listener {
//     fn default() -> Self {
//         Self {
//             pos: Default::default(),
//             ori: Default::default(),
//             ear_left_rpos: Vec3::unit_x(),
//             ear_right_rpos: -Vec3::unit_x(),
//         }
//     }
// }

pub fn to_decibels(amplitude: f32) -> Decibels {
    if amplitude <= 0.001 {
        Decibels::SILENCE
    } else if amplitude == 1.0 {
        Decibels::IDENTITY
    } else {
        Decibels(amplitude.log10() * 20.0)
    }
}

struct Tracks {
    music: TrackHandle,
    ui: TrackHandle,
    sfx: TrackHandle,
    ambience: TrackHandle,
}

#[derive(Clone, Copy, Debug, Display)]
pub enum SfxChannelSettings {
    Low,
    Medium,
    High,
}

impl SfxChannelSettings {
    pub fn from_str_slice(str: &str) -> Self {
        match str {
            "Low" => SfxChannelSettings::Low,
            "Medium" => SfxChannelSettings::Medium,
            "High" => SfxChannelSettings::High,
            _ => SfxChannelSettings::High,
        }
    }

    pub fn to_usize(&self) -> usize {
        match self {
            SfxChannelSettings::Low => 16,
            SfxChannelSettings::Medium => 32,
            SfxChannelSettings::High => 64,
        }
    }
}

struct Effects {
    sfx: FilterHandle,
    ambience: FilterHandle,
}

#[derive(Default)]
struct Channels {
    music: Vec<MusicChannel>,
    ambience: Vec<AmbienceChannel>,
    sfx: Vec<SfxChannel>,
    ui: Vec<UiChannel>,
}

impl Channels {
    /// Gets the music channel matching the given tag, of which there should be
    /// only one, if any.
    fn get_music_channel(&mut self, channel_tag: MusicChannelTag) -> Option<&mut MusicChannel> {
        self.music.iter_mut().find(|c| c.get_tag() == channel_tag)
    }

    /// Retrive an empty sfx channel from the list
    fn get_sfx_channel(&mut self) -> Option<&mut SfxChannel> {
        self.sfx.iter_mut().find(|c| c.is_done())
    }

    /// Retrive an empty UI channel from the list
    fn get_ui_channel(&mut self) -> Option<&mut UiChannel> {
        self.ui.iter_mut().find(|c| c.is_done())
    }

    /// Retrieves the channel currently having the given tag
    /// If no channel with the given tag is found, returns None
    fn get_ambience_channel(
        &mut self,
        channel_tag: AmbienceChannelTag,
    ) -> Option<&mut AmbienceChannel> {
        self.ambience
            .iter_mut()
            .find(|channel| channel.get_tag() == channel_tag)
    }

    fn count_active(&self) -> ActiveChannels {
        ActiveChannels {
            music: self.music.iter().filter(|c| !c.is_done()).count(),
            ambience: self.ambience.iter().filter(|c| c.is_active()).count(),
            sfx: self.sfx.iter().filter(|c| !c.is_done()).count(),
            ui: self.ui.iter().filter(|c| !c.is_done()).count(),
        }
    }
}

#[derive(Default)]
pub struct ActiveChannels {
    pub music: usize,
    pub ambience: usize,
    pub sfx: usize,
    pub ui: usize,
}

#[derive(Default)]
struct Volumes {
    sfx: f32,
    ambience: f32,
    music: f32,
    master: f32,
}

struct ListenerInstance {
    handle: ListenerHandle,
    pos: Vec3<f32>,
    ori: Vec3<f32>,
}

struct AudioFrontendInner {
    manager: AudioManager,
    tracks: Tracks,
    effects: Effects,
    channels: Channels,
    listener: ListenerInstance,
    clock: ClockHandle,
}

enum AudioCreationError {
    Manager(<CpalBackend as backend::Backend>::Error),
    Clock(kira::ResourceLimitReached),
    Track(kira::ResourceLimitReached),
    Listener(kira::ResourceLimitReached),
}

impl AudioFrontendInner {
    fn new(
        num_sfx_channels: usize,
        num_ui_channels: usize,
        buffer_size: usize,
        device: Option<Device>,
        config: Option<StreamConfig>,
    ) -> Result<Self, AudioCreationError> {
        let backend_settings = CpalBackendSettings { device, config };
        let manager_settings = AudioManagerSettings {
            internal_buffer_size: buffer_size,
            backend_settings,
            ..Default::default()
        };
        let mut manager = AudioManager::<CpalBackend>::new(manager_settings)
            .map_err(AudioCreationError::Manager)?;

        let mut clock = manager
            .add_clock(ClockSpeed::TicksPerSecond(1.0))
            .map_err(AudioCreationError::Clock)?;
        clock.start();

        let mut sfx_track_builder = TrackBuilder::new();
        let mut ambience_track_builder = TrackBuilder::new();

        let effects = Effects {
            sfx: sfx_track_builder.add_effect(FilterBuilder::new().cutoff(Value::Fixed(20000.0))),
            ambience: ambience_track_builder
                .add_effect(FilterBuilder::new().cutoff(Value::Fixed(20000.0))),
        };

        let listener_handle = manager
            .add_listener(Vec3::zero(), Quaternion::identity())
            .map_err(AudioCreationError::Listener)?;

        let listener = ListenerInstance {
            handle: listener_handle,
            pos: Vec3::zero(),
            ori: Vec3::unit_x(),
        };

        let mut tracks = Tracks {
            music: manager
                .add_sub_track(TrackBuilder::new())
                .map_err(AudioCreationError::Track)?,
            ui: manager
                .add_sub_track(TrackBuilder::new())
                .map_err(AudioCreationError::Track)?,
            sfx: manager
                .add_sub_track(sfx_track_builder)
                .map_err(AudioCreationError::Track)?,
            ambience: manager
                .add_sub_track(ambience_track_builder)
                .map_err(AudioCreationError::Track)?,
        };

        let mut channels = Channels::default();

        for _ in 0..num_sfx_channels {
            if let Ok(channel) = SfxChannel::new(&mut tracks.sfx, listener.handle.id()) {
                channels.sfx.push(channel);
            } else {
                warn!("Cannot create sfx channel")
            }
        }

        for _ in 0..num_ui_channels {
            if let Ok(channel) = UiChannel::new(&mut tracks.ui) {
                channels.ui.push(channel);
            } else {
                warn!("Cannot create ui channel")
            }
        }

        Ok(Self {
            manager,
            tracks,
            effects,
            channels,
            listener,
            clock,
        })
    }

    fn manager(&mut self) -> &mut AudioManager { &mut self.manager }

    fn clock(&self) -> &ClockHandle { &self.clock }

    fn create_music_channel(&mut self, channel_tag: MusicChannelTag) {
        let channel = MusicChannel::new(&mut self.tracks.music);
        match channel {
            Ok(mut next_music_channel) => {
                next_music_channel.set_volume(1.0);
                next_music_channel.set_tag(channel_tag);
                self.channels.music.push(next_music_channel);
            },
            Err(e) => error!(
                ?e,
                "Failed to crate new music channel, music may fail playing"
            ),
        }
    }

    /// Adds a new ambience channel of the given tag at zero volume
    fn new_ambience_channel(&mut self, channel_tag: AmbienceChannelTag) {
        let channel = AmbienceChannel::new(channel_tag, 0.0, &mut self.tracks.ambience, true);
        match channel {
            Ok(ambience_channel) => self.channels.ambience.push(ambience_channel),
            Err(e) => error!(
                ?e,
                "Failed to crate new ambience channel, sounds may fail playing"
            ),
        }
    }
}

/// Holds information about the system audio devices and internal channels used
/// for sfx and music playback. An instance of `AudioFrontend` is used by
/// Voxygen's [`GlobalState`](../struct.GlobalState.html#structfield.audio) to
/// provide access to devices and playback control in-game
///
/// TODO: Use a listener struct (like the one commented out above) instead of
/// keeping all listener data in the AudioFrontend struct. Will be helpful when
/// we do more with spatial audio.
pub struct AudioFrontend {
    inner: Option<AudioFrontendInner>,

    pub subtitles_enabled: bool,
    pub subtitles: VecDeque<Subtitle>,

    volumes: Volumes,
    music_spacing: f32,
    pub combat_music_enabled: bool,

    mtm: AssetHandle<MusicTransitionManifest>,
}

impl AudioFrontend {
    pub fn new(
        num_sfx_channels: usize,
        num_ui_channels: usize,
        subtitles: bool,
        combat_music_enabled: bool,
        buffer_size: usize,
        set_samplerate: Option<u32>,
    ) -> Self {
        // Generate a supported config if the default samplerate is too high or is
        // manually set.
        let mut device = cpal::default_host().default_output_device();
        let mut supported_config = None;
        let mut samplerate = 44100;
        if let Some(device) = device.as_mut() {
            if let Ok(default_output_config) = device.default_output_config() {
                info!(
                    "Current default samplerate: {:?}",
                    default_output_config.sample_rate().0
                );
                samplerate = default_output_config.sample_rate().0;
                if samplerate > 48000 && set_samplerate.is_none() {
                    warn!(
                        "Current default samplerate is higher than 48000; attempting to lower \
                         samplerate"
                    );
                    let supported_configs = device.supported_output_configs();
                    if let Ok(supported_configs) = supported_configs {
                        let best_config = supported_configs.max_by(|x, y| {
                            SupportedStreamConfigRange::cmp_default_heuristics(x, y)
                        });
                        if let Some(best_config) = best_config {
                            warn!("Attempting to change samplerate to 48khz");
                            supported_config = best_config.try_with_sample_rate(SampleRate(48000));
                            if supported_config.is_none() {
                                warn!("Attempting to change samplerate to 44.1khz");
                                supported_config =
                                    best_config.try_with_sample_rate(SampleRate(44100));
                            }
                            if supported_config.is_none() {
                                warn!("Could not change samplerate, using default")
                            }
                        }
                    }
                } else if let Some(set_samplerate) = set_samplerate {
                    let supported_configs = device.supported_output_configs();
                    if let Ok(supported_configs) = supported_configs {
                        let best_config = supported_configs.max_by(|x, y| {
                            SupportedStreamConfigRange::cmp_default_heuristics(x, y)
                        });
                        if let Some(best_config) = best_config {
                            warn!("Attempting to force samplerate to {:?}", set_samplerate);
                            supported_config =
                                best_config.try_with_sample_rate(SampleRate(set_samplerate));
                            if supported_config.is_none() {
                                error!(
                                    "Could not set samplerate to {:?}, falling back to default.",
                                    set_samplerate
                                );
                            }
                        }
                    }
                }
            }
        }
        let mut config = None;
        if let Some(supported_config) = supported_config {
            info!(
                "Samplerate is {:?}",
                supported_config.config().sample_rate.0
            );
            config = Some(supported_config.config())
        } else {
            info!("Samplerate is {:?}", samplerate)
        }
        let inner = AudioFrontendInner::new(
            num_sfx_channels,
            num_ui_channels,
            buffer_size,
            device,
            config,
        )
        .inspect_err(|err| match err {
            AudioCreationError::Manager(e) => {
                #[cfg(unix)]
                error!(
                    ?e,
                    "failed to construct audio frontend manager. Is `pulseaudio-alsa` installed?"
                );
                #[cfg(not(unix))]
                error!(?e, "failed to construct audio frontend manager.");
            },
            AudioCreationError::Clock(e) => {
                error!(?e, "Failed to construct audio frontend clock.")
            },
            AudioCreationError::Track(e) => {
                error!(?e, "Failed to construct audio frontend track.")
            },
            AudioCreationError::Listener(e) => {
                error!(?e, "Failed to construct audio frontend listener.")
            },
        })
        .ok();

        if let Some(inner) = inner {
            Self {
                inner: Some(inner),
                volumes: Volumes::default(),
                music_spacing: 1.0,
                mtm: AssetExt::load_expect("voxygen.audio.music_transition_manifest"),
                subtitles: VecDeque::new(),
                subtitles_enabled: subtitles,
                combat_music_enabled,
            }
        } else {
            Self {
                inner: None,
                volumes: Volumes::default(),
                music_spacing: 1.0,
                mtm: AssetExt::load_expect("voxygen.audio.music_transition_manifest"),
                subtitles: VecDeque::new(),
                subtitles_enabled: subtitles,
                combat_music_enabled,
            }
        }
    }

    /// Construct in `no-audio` mode for debugging
    pub fn no_audio() -> Self {
        Self {
            inner: None,
            music_spacing: 1.0,
            volumes: Volumes::default(),
            mtm: AssetExt::load_expect("voxygen.audio.music_transition_manifest"),
            subtitles: VecDeque::new(),
            subtitles_enabled: false,
            combat_music_enabled: false,
        }
    }

    /// Drop any unused music channels, ambience channels, and reset the tags of
    /// unused UI channels.
    pub fn maintain(&mut self) {
        if let Some(inner) = &mut self.inner {
            inner.channels.music.retain(|c| !c.is_done());
            inner.channels.ambience.retain(|c| !c.is_stopped());
            // Also set any unused sfx channels to 0 volume to prevent popping in some
            // cases.
            inner.channels.sfx.iter_mut().for_each(|c| {
                if c.is_done() {
                    c.set_volume(0.0);
                }
            });
            inner.channels.ui.iter_mut().for_each(|c| {
                if c.is_done() {
                    c.tag = None
                }
            });
        }
    }

    pub fn get_clock(&self) -> Option<&ClockHandle> { self.inner.as_ref().map(|i| i.clock()) }

    pub fn get_clock_time(&self) -> Option<ClockTime> { self.get_clock().map(|clock| clock.time()) }

    /// Returns [music channels, ambience channels, sfx channels, ui channels]
    pub fn get_num_active_channels(&self) -> ActiveChannels {
        self.inner
            .as_ref()
            .map(|i| i.channels.count_active())
            .unwrap_or_default()
    }

    pub fn get_cpu_usage(&mut self) -> f32 {
        if let Some(inner) = self.inner.as_mut() {
            inner.manager.backend_mut().pop_cpu_usage().unwrap_or(0.0)
        } else {
            0.0
        }
    }

    /// Play a music file with the given tag. Pass in the length of the track in
    /// seconds.
    fn play_music(&mut self, sound: &str, channel_tag: MusicChannelTag, length: f32) {
        if self.music_enabled()
            && let Some(inner) = &mut self.inner
        {
            let mtm = self.mtm.read();

            if let Some(current_channel) = inner.channels.music.iter_mut().find(|c| !c.is_done()) {
                let (fade_out, _fade_in) = mtm
                    .fade_timings
                    .get(&(current_channel.get_tag(), channel_tag))
                    .unwrap_or(&(1.0, 1.0));
                current_channel.fade_out(*fade_out, None);
            }

            let now = inner.clock().time();

            let channel = match inner.channels.get_music_channel(channel_tag) {
                Some(c) => c,
                None => {
                    inner.create_music_channel(channel_tag);
                    inner
                        .channels
                        .music
                        .last_mut()
                        .expect("We just created this")
                },
            };

            let (fade_out, fade_in) = mtm
                .fade_timings
                .get(&(channel.get_tag(), channel_tag))
                .unwrap_or(&(1.0, 0.1));
            let source = load_ogg(sound, true);
            channel.stop(Some(*fade_out), None);
            channel.set_length(length);
            channel.set_tag(channel_tag);
            channel.set_loop_data(false, LoopPoint::Start, LoopPoint::End);
            channel.play(source, now, Some(*fade_in), Some(*fade_out));
        }
    }

    /// Turn on or off looping
    pub fn set_loop(&mut self, channel_tag: MusicChannelTag, sound_loops: bool) {
        if let Some(inner) = self.inner.as_mut() {
            let channel = inner.channels.get_music_channel(channel_tag);
            if let Some(channel) = channel {
                let loop_data = channel.get_loop_data();
                channel.set_loop_data(sound_loops, loop_data.1, loop_data.2);
            }
        }
    }

    /// Loops music from start point to end point in seconds
    pub fn set_loop_points(&mut self, channel_tag: MusicChannelTag, start: f32, end: f32) {
        if let Some(inner) = self.inner.as_mut() {
            let channel = inner.channels.get_music_channel(channel_tag);
            if let Some(channel) = channel {
                channel.set_loop_data(
                    true,
                    LoopPoint::Point(start as f64),
                    LoopPoint::Point(end as f64),
                );
            }
        }
    }

    /// Find sound based on given trigger_item.
    /// Randomizes if multiple sounds are found.
    /// Errors if no sounds are found.
    /// Returns (file, threshold, subtitle)
    pub fn get_sfx_file<'a>(
        trigger_item: Option<(&'a SfxEvent, &'a SfxTriggerItem)>,
    ) -> Option<(&'a str, f32, Option<&'a str>)> {
        trigger_item.map(|(event, item)| {
            let file = match item.files.len() {
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

            // NOTE: Threshold here is meant to give subtitles some idea of the duration of
            // the audio, it doesn't have to be perfect but in the future, if possible we
            // might want to switch it out for the actual duration.
            (file, item.threshold, item.subtitle.as_deref())
        })
    }

    /// Set the cutoff of the filter affecting all spatial sfx
    pub fn set_sfx_master_filter(&mut self, frequency: u32) {
        if let Some(inner) = self.inner.as_mut() {
            inner
                .effects
                .sfx
                .set_cutoff(Value::Fixed(frequency as f64), Tween::default());
        }
    }

    /// Play an sfx file given the position and SfxEvent at the given volume
    /// (default 1.0)
    pub fn emit_sfx(
        &mut self,
        trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
        emitter_pos: Vec3<f32>,
        volume: Option<f32>,
        player_pos: Vec3<f32>,
        emitter_pos_entity: Option<EcsEntity>,
    ) {
        if let Some((sfx_file, dur, subtitle)) = Self::get_sfx_file(trigger_item) {
            self.emit_subtitle(subtitle, Some(emitter_pos), dur);
            // Play sound in empty channel at given position
            if self.sfx_enabled()
                && let Some(inner) = self.inner.as_mut()
                && let Some(channel) = inner.channels.get_sfx_channel()
            {
                let sound = load_ogg(sfx_file, false);
                channel.pos_entity = emitter_pos_entity;
                channel.update(emitter_pos, player_pos);

                let source = sound.volume(to_decibels(volume.unwrap_or(1.0) * 5.0));
                channel.play(source);
            }
        } else {
            warn!(
                "Missing sfx trigger config for sfx event: {:?}; {:?}",
                trigger_item,
                backtrace::Backtrace::new(),
            );
        }
    }

    /// Plays a sfx non-spatially at the given volume (default 1.0); doesn't
    /// need a position
    pub fn emit_ui_sfx(
        &mut self,
        trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
        volume: Option<f32>,
        tag: Option<channel::UiChannelTag>,
    ) {
        if let Some((sfx_file, dur, subtitle)) = Self::get_sfx_file(trigger_item) {
            self.emit_subtitle(subtitle, None, dur);

            // Play sound in empty channel
            if self.sfx_enabled()
                && let Some(inner) = self.inner.as_mut()
                && !inner
                    .channels
                    .ui
                    .iter()
                    .any(|c| tag.is_some() && c.tag == tag)
                && let Some(channel) = inner.channels.get_ui_channel()
            {
                let sound = load_ogg(sfx_file, false).volume(to_decibels(volume.unwrap_or(1.0)));
                channel.play(sound, tag);
            }
        } else {
            warn!("Missing sfx trigger config for ui sfx event.",);
        }
    }

    /// Push a subtitle to the subtitle queue
    pub fn emit_subtitle(
        &mut self,
        subtitle: Option<&str>,
        position: Option<Vec3<f32>>,
        duration: f32,
    ) {
        if self.subtitles_enabled {
            if let Some(subtitle) = subtitle {
                self.subtitles.push_back(Subtitle {
                    localization: subtitle.to_string(),
                    position,
                    show_for: duration as f64,
                });
                if self.subtitles.len() > 10 {
                    self.subtitles.pop_front();
                }
            }
        }
    }

    /// Set the cutoff of the filter affecting all ambience
    pub fn set_ambience_master_filter(&mut self, frequency: u32) {
        if let Some(inner) = self.inner.as_mut() {
            inner
                .effects
                .ambience
                .set_cutoff(Value::Fixed(frequency as f64), Tween::default());
        }
    }

    /// Plays an ambience sound that loops in the channel with a given tag
    pub fn play_ambience_looping(
        &mut self,
        channel_tag: AmbienceChannelTag,
        sound: &str,
        start: usize,
        end: usize,
    ) {
        if self.ambience_enabled()
            && let Some(inner) = self.inner.as_mut()
            && let Some(channel) = inner.channels.get_ambience_channel(channel_tag)
        {
            let source = load_ogg(sound, true).loop_region(
                kira::sound::PlaybackPosition::Samples(start)
                    ..kira::sound::PlaybackPosition::Samples(end),
            );
            channel.play(source, Some(1.0), None);
        }
    }

    /// Plays an ambience sound once at the given volume after the given delay.
    /// Make sure it uses a channel tag that does not change the volume of its
    /// channel. Currently, ambience oneshots use the Sfx file system
    pub fn play_ambience_oneshot(
        &mut self,
        channel_tag: AmbienceChannelTag,
        trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
        volume: Option<f32>,
        delay: Option<f32>,
    ) {
        if self.ambience_enabled()
            && trigger_item.is_some()
            && let Some(inner) = self.inner.as_mut()
            && let Some(channel) = inner.channels.get_ambience_channel(channel_tag)
        {
            let sound = AudioFrontend::get_sfx_file(trigger_item)
                .unwrap_or(("", 0.0, Some("")))
                .0;
            let source = load_ogg(sound, false)
                .loop_region(None)
                .volume(to_decibels(volume.unwrap_or(1.0)));
            channel.fade_to(1.0, 0.0);
            channel.play(source, None, delay);
        }
    }

    pub fn set_listener_pos(&mut self, pos: Vec3<f32>, ori: Vec3<f32>) {
        if let Some(inner) = self.inner.as_mut() {
            let tween = Tween {
                duration: Duration::from_secs_f32(0.01),
                ..Default::default()
            };

            inner.listener.pos = pos;
            inner.listener.ori = ori;

            inner.listener.handle.set_position(pos, tween);

            let ori_quat = Ori::from(ori).to_quat();
            inner
                .listener
                .handle
                .set_orientation(ori_quat.normalized(), tween);
        }
    }

    pub fn get_listener(&mut self) -> Option<&mut ListenerHandle> {
        self.inner.as_mut().map(|i| &mut i.listener.handle)
    }

    pub fn get_listener_pos(&self) -> Vec3<f32> {
        self.inner
            .as_ref()
            .map(|i| i.listener.pos)
            .unwrap_or_default()
    }

    pub fn get_listener_ori(&self) -> Vec3<f32> {
        self.inner
            .as_ref()
            .map(|i| i.listener.ori)
            .unwrap_or_else(Vec3::unit_x)
    }

    /// Switches the playing music to the title music, which is pinned to a
    /// specific sound file (veloren_title_tune.ogg)
    pub fn play_title_music(&mut self) {
        if self.music_enabled() {
            self.play_music(
                "voxygen.audio.soundtrack.veloren_title_tune",
                MusicChannelTag::TitleMusic,
                43.0,
            );
            self.set_loop(MusicChannelTag::TitleMusic, true);
        }
    }

    /// Retrieves the current setting for master volume
    pub fn get_master_volume(&self) -> f32 { self.volumes.master }

    /// Retrieves the current setting for music volume
    pub fn get_music_volume(&self) -> f32 { self.volumes.music }

    /// Retrieves the current setting for ambience volume
    pub fn get_ambience_volume(&self) -> f32 { self.volumes.ambience }

    /// Retrieves the current setting for sfx volume
    pub fn get_sfx_volume(&self) -> f32 { self.volumes.sfx }

    /// Returns false if volume is 0 or the mute is on
    pub fn music_enabled(&self) -> bool { self.get_music_volume() > 0.0 }

    /// Returns false if volume is 0 or the mute is on
    pub fn ambience_enabled(&self) -> bool { self.get_ambience_volume() > 0.0 }

    /// Returns false if volume is 0 or the mute is on
    pub fn sfx_enabled(&self) -> bool { self.get_sfx_volume() > 0.0 }

    pub fn set_music_volume(&mut self, music_volume: f32) {
        self.volumes.music = music_volume;

        if let Some(inner) = self.inner.as_mut() {
            inner
                .tracks
                .music
                .set_volume(to_decibels(music_volume), Tween::default())
        }
    }

    pub fn set_ambience_volume(&mut self, ambience_volume: f32) {
        self.volumes.ambience = ambience_volume;

        if let Some(inner) = self.inner.as_mut() {
            inner
                .tracks
                .ambience
                .set_volume(to_decibels(ambience_volume), Tween::default())
        }
    }

    /// Sets the volume for both spatial sfx and UI (might separate these
    /// controls later)
    pub fn set_sfx_volume(&mut self, sfx_volume: f32) {
        self.volumes.sfx = sfx_volume;

        if let Some(inner) = self.inner.as_mut() {
            inner
                .tracks
                .sfx
                .set_volume(to_decibels(sfx_volume), Tween::default())
        }
    }

    pub fn set_music_spacing(&mut self, multiplier: f32) { self.music_spacing = multiplier }

    pub fn set_subtitles(&mut self, enabled: bool) { self.subtitles_enabled = enabled }

    /// Updates volume of the master track
    pub fn set_master_volume(&mut self, master_volume: f32) {
        self.volumes.master = master_volume;

        if let Some(inner) = self.inner.as_mut() {
            inner
                .manager()
                .main_track()
                .set_volume(to_decibels(master_volume), Tween::default());
        }
    }

    pub fn stop_all_ambience(&mut self) {
        if let Some(inner) = self.inner.as_mut() {
            for channel in &mut inner.channels.ambience {
                channel.stop(None, None);
            }
        }
    }

    pub fn stop_all_music(&mut self) {
        if let Some(inner) = self.inner.as_mut() {
            for channel in &mut inner.channels.music {
                channel.stop(None, None);
            }
        }
    }

    pub fn stop_all_sfx(&mut self) {
        if let Some(inner) = self.inner.as_mut() {
            for channel in &mut inner.channels.sfx {
                channel.stop();
            }
            for channel in &mut inner.channels.ui {
                channel.stop();
            }
        }
    }

    pub fn set_num_sfx_channels(&mut self, channels: usize) {
        if let Some(inner) = self.inner.as_mut() {
            inner.channels.sfx = Vec::new();
            for _ in 0..channels {
                if let Ok(channel) =
                    SfxChannel::new(&mut inner.tracks.sfx, inner.listener.handle.id())
                {
                    inner.channels.sfx.push(channel);
                } else {
                    warn!("Cannot create sfx channel")
                }
            }
        }
    }

    pub fn get_num_music_channels(&self) -> usize {
        self.inner
            .as_ref()
            .map(|i| i.channels.music.len())
            .unwrap_or(0)
    }

    pub fn get_num_ambience_channels(&self) -> usize {
        self.inner
            .as_ref()
            .map(|i| i.channels.ambience.len())
            .unwrap_or(0)
    }
}
