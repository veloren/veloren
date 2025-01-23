//! Handles audio device detection and playback of sound effects and music

pub mod ambience;
pub mod channel;
pub mod fader;
pub mod music;
pub mod sfx;
pub mod soundcache;

use anim::vek::Quaternion;
use channel::{
    AmbienceChannel, AmbienceChannelTag, LoopPoint, MusicChannel, MusicChannelTag, SfxChannel,
    UiChannel,
};
use kira::{
    Volume,
    clock::{ClockHandle, ClockSpeed, ClockTime},
    effect::filter::{FilterBuilder, FilterHandle},
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    spatial::{
        emitter::EmitterSettings,
        listener::{ListenerHandle, ListenerSettings},
        scene::{SpatialSceneHandle, SpatialSceneSettings},
    },
    track::{TrackBuilder, TrackHandle},
    tween::{Easing, Tween, Value},
};
use music::MusicTransitionManifest;
use sfx::{SfxEvent, SfxTriggerItem};
use soundcache::{AnySoundData, AnySoundHandle, load_ogg};
use std::{collections::VecDeque, time::Duration};
use tracing::{debug, error, warn};

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

pub enum MasterEffect {
    SfxFilter(FilterHandle),
    AmbienceFilter(FilterHandle),
}

struct Tracks {
    music: TrackHandle,
    ui: TrackHandle,
    sfx: TrackHandle,
    ambience: TrackHandle,
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

struct SoundPlayer {
    manager: AudioManager,
    clock: ClockHandle,
}

impl SoundPlayer {
    /// Play a sound using the AudioManager
    fn play_sound(
        &mut self,
        mut source: AnySoundData,
        fade_in: Option<f32>,
        delay: Option<f32>,
    ) -> Option<AnySoundHandle> {
        if let Some(fade_in) = fade_in {
            let fade_in_tween = Tween {
                duration: Duration::from_secs_f32(fade_in),
                ..Default::default()
            };
            source = source.fade_in_tween(fade_in_tween);
        }

        if let Some(delay) = delay {
            source = source.start_time(self.clock.time() + delay as f64);
        }

        self.manager
            .play(source)
            .inspect_err(|e| error!(?e, "Failed to play sound."))
            .ok()
    }
}

struct AudioFrontendInner {
    sound_player: SoundPlayer,
    #[expect(dead_code)]
    scene: SpatialSceneHandle,
    tracks: Tracks,
    effects: Effects,
    channels: Channels,
    listener: ListenerInstance,
}

enum AudioCreationError {
    Manager(<DefaultBackend as kira::manager::backend::Backend>::Error),
    Clock(kira::ResourceLimitReached),
    Track(kira::ResourceLimitReached),
    Scene(kira::ResourceLimitReached),
}

impl AudioFrontendInner {
    fn new(num_sfx_channels: usize, num_ui_channels: usize) -> Result<Self, AudioCreationError> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
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

        let tracks = Tracks {
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

        let mut scene = manager
            .add_spatial_scene(SpatialSceneSettings::new())
            .map_err(AudioCreationError::Scene)?;
        let listener_handle = scene
            .add_listener(
                Vec3::zero(),
                Quaternion::identity(),
                ListenerSettings::new().track(tracks.sfx.id()),
            )
            .map_err(AudioCreationError::Scene)?;

        let listener = ListenerInstance {
            handle: listener_handle,
            pos: Vec3::zero(),
            ori: Vec3::unit_x(),
        };

        for _ in 0..num_sfx_channels {
            let emitter = scene
                .add_emitter(
                    Vec3::zero(),
                    EmitterSettings::new()
                        .persist_until_sounds_finish(true)
                        .distances([1.0, 200.0])
                        .attenuation_function(Some(Easing::OutPowf(0.45))),
                )
                .map_err(AudioCreationError::Scene)?;

            channels.sfx.push(SfxChannel::new(emitter));
        }

        channels.ui.resize_with(num_ui_channels, || {
            UiChannel::new(&mut manager, tracks.ui.id())
        });

        Ok(Self {
            sound_player: SoundPlayer { manager, clock },
            scene,
            tracks,
            effects,
            channels,
            listener,
        })
    }

    fn manager(&mut self) -> &mut AudioManager { &mut self.sound_player.manager }

    fn clock(&self) -> &ClockHandle { &self.sound_player.clock }

    fn create_music_channel(&mut self, channel_tag: MusicChannelTag) {
        let parent_track = self.tracks.music.id();

        let mut next_music_channel = MusicChannel::new(self.manager(), parent_track);
        next_music_channel.set_volume(1.0);
        next_music_channel.set_tag(channel_tag);
        self.channels.music.push(next_music_channel);
    }

    /// Adds a new ambience channel of the given tag at zero volume
    fn new_ambience_channel(&mut self, channel_tag: AmbienceChannelTag) {
        let parent_track = self.tracks.ambience.id();
        match AmbienceChannel::new(channel_tag, 0.0, self.manager(), parent_track, true) {
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
    ) -> Self {
        let inner = AudioFrontendInner::new(num_sfx_channels, num_ui_channels)
            .inspect_err(|err| match err {
                AudioCreationError::Manager(e) => {
                    #[cfg(unix)]
                    error!(
                        ?e,
                        "failed to construct audio frontend manager. Is `pulseaudio-alsa` \
                         installed?"
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
                AudioCreationError::Scene(e) => {
                    error!(?e, "Failed to construct audio frontend scene.")
                },
            })
            .ok();

        Self {
            inner,
            volumes: Volumes::default(),
            music_spacing: 1.0,
            mtm: AssetExt::load_expect("voxygen.audio.music_transition_manifest"),
            subtitles: VecDeque::new(),
            subtitles_enabled: subtitles,
            combat_music_enabled,
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

    /// Drop any unused music channels
    pub fn maintain(&mut self) {
        if let Some(inner) = &mut self.inner {
            inner.channels.music.retain(|c| !c.is_done());
            inner.channels.ambience.retain(|c| !c.is_stopped());
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
            let track = channel.get_track();
            let source = load_ogg(sound, true).output_destination(track.unwrap());
            channel.stop(Some(*fade_out), None);
            channel.set_length(length);
            channel.set_tag(channel_tag);
            channel.set_loop_data(false, LoopPoint::Start, LoopPoint::End);
            let handle = inner
                .sound_player
                .play_sound(source, Some(*fade_in), Some(*fade_out));
            channel.set_source(handle);
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

    /// Set the cutoff of the filter affecting all spatial sfx
    pub fn set_ambience_master_filter(&mut self, frequency: u32) {
        if let Some(inner) = self.inner.as_mut() {
            inner
                .effects
                .ambience
                .set_cutoff(Value::Fixed(frequency as f64), Tween::default());
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
        position: Vec3<f32>,
        volume: Option<f32>,
    ) {
        if let Some((sfx_file, dur, subtitle)) = Self::get_sfx_file(trigger_item) {
            self.emit_subtitle(subtitle, Some(position), dur);
            // Play sound in empty channel at given position
            if self.sfx_enabled()
                && let Some(inner) = self.inner.as_mut()
                && let Some(channel) = inner.channels.get_sfx_channel()
            {
                let sound = load_ogg(sfx_file, false);
                channel.update(position);
                let emitter = channel.get_emitter();

                let source = sound
                    .volume(Volume::Amplitude((volume.unwrap_or(1.0) * 5.0) as f64))
                    .output_destination(emitter);
                let handle = inner.sound_player.play_sound(source, None, None);
                channel.set_source(handle);
            }
        } else {
            warn!(
                "Missing sfx trigger config for sfx event at position: {:?}",
                position
            );
        }
    }

    /// Plays a sfx non-spatially at the given volume (default 1.0); doesn't
    /// need a position
    pub fn emit_ui_sfx(
        &mut self,
        trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
        volume: Option<f32>,
    ) {
        if let Some((sfx_file, dur, subtitle)) = Self::get_sfx_file(trigger_item) {
            self.emit_subtitle(subtitle, None, dur);

            // Play sound in empty channel
            if self.sfx_enabled()
                && let Some(inner) = self.inner.as_mut()
                && let Some(channel) = inner.channels.get_ui_channel()
            {
                let sound = load_ogg(sfx_file, false)
                    .volume(Volume::Amplitude(volume.unwrap_or(1.0) as f64));
                let handle = inner.sound_player.play_sound(sound, None, None);
                channel.set_source(handle);
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

    /// Plays an ambience sound that loops in the channel with a given tag
    pub fn play_ambience_looping(&mut self, channel_tag: AmbienceChannelTag, sound: &str) {
        if self.ambience_enabled()
            && let Some(inner) = self.inner.as_mut()
            && let Some(channel) = inner.channels.get_ambience_channel(channel_tag)
        {
            let source = load_ogg(sound, true)
                .loop_region(0.0..)
                .output_destination(channel.get_track());
            channel.set_looping(true);
            let handle = inner.sound_player.play_sound(source, Some(1.0), None);
            channel.set_source(handle);
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
                .output_destination(channel.get_track())
                .volume(Volume::Amplitude(volume.unwrap_or(1.0) as f64));
            channel.set_looping(false);
            channel.fade_to(1.0, 0.0);
            let handle = inner.sound_player.play_sound(source, None, delay);
            channel.set_source(handle);
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
                .set_volume(Volume::Amplitude(music_volume as f64), Tween::default())
        }
    }

    pub fn set_ambience_volume(&mut self, ambience_volume: f32) {
        self.volumes.ambience = ambience_volume;

        if let Some(inner) = self.inner.as_mut() {
            inner
                .tracks
                .ambience
                .set_volume(Volume::Amplitude(ambience_volume as f64), Tween::default())
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
                .set_volume(Volume::Amplitude(sfx_volume as f64), Tween::default())
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
                .set_volume(Volume::Amplitude(master_volume as f64), Tween::default());
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
