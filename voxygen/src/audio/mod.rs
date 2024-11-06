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
    clock::{ClockHandle, ClockSpeed, ClockTime},
    effect::filter::{FilterBuilder, FilterHandle},
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
    spatial::{
        emitter::EmitterSettings,
        listener::{ListenerHandle, ListenerSettings},
        scene::{SpatialSceneHandle, SpatialSceneSettings},
    },
    track::{TrackBuilder, TrackHandle},
    tween::{Easing, Tween, Value},
    Volume,
};
use music::MusicTransitionManifest;
use sfx::{SfxEvent, SfxTriggerItem};
use soundcache::load_ogg;
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
    manager: Option<AudioManager>,
    spatial_manager: Option<AudioManager>,
    scene: Option<SpatialSceneHandle>,
    clock: Option<ClockHandle>,
    master_tracks: Vec<TrackHandle>,
    master_effects: Vec<MasterEffect>,

    music_channels: Vec<MusicChannel>,
    ambience_channels: Vec<AmbienceChannel>,
    sfx_channels: Vec<SfxChannel>,
    ui_channels: Vec<UiChannel>,
    sfx_volume: f32,
    ambience_volume: f32,
    music_volume: f32,
    master_volume: f32,
    music_spacing: f32,
    listener: Option<ListenerHandle>,
    listener_pos: Vec3<f32>,
    listener_ori: Vec3<f32>,

    pub subtitles_enabled: bool,
    pub subtitles: VecDeque<Subtitle>,

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
        let mut master_tracks = Vec::with_capacity(3);
        let mut master_effects = Vec::new();
        let mut sfx_channels = Vec::with_capacity(num_sfx_channels);
        let mut ui_channels = Vec::with_capacity(num_ui_channels);
        let mut scene: Option<SpatialSceneHandle> = None;
        let mut listener: Option<ListenerHandle> = None;
        let mut clock: Option<ClockHandle> = None;

        let mut manager = match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
        {
            Ok(m) => Some(m),
            Err(e) => {
                #[cfg(unix)]
                error!(
                    ?e,
                    "failed to construct audio frontend. Is `pulseaudio-alsa` installed?"
                );
                #[cfg(not(unix))]
                error!(?e, "failed to construct audio frontend.");
                None
            },
        };
        let mut spatial_manager_settings = AudioManagerSettings::default();
        let sfx_filter_builder = FilterBuilder::new().cutoff(Value::Fixed(20000.0));
        let mut sfx_track_builder = TrackBuilder::new();
        let sfx_filter = sfx_track_builder.add_effect(sfx_filter_builder);
        spatial_manager_settings.main_track_builder = sfx_track_builder;
        let mut spatial_manager =
            match AudioManager::<DefaultBackend>::new(spatial_manager_settings) {
                Ok(m) => Some(m),
                Err(e) => {
                    #[cfg(unix)]
                    error!(
                        ?e,
                        "failed to construct audio frontend. Is `pulseaudio-alsa` installed?"
                    );
                    #[cfg(not(unix))]
                    error!(?e, "failed to construct audio frontend.");
                    None
                },
            };
        master_effects.push(MasterEffect::SfxFilter(sfx_filter));

        if let Some(manager) = manager.as_mut() {
            let mut clock_result = manager.add_clock(ClockSpeed::TicksPerSecond(1.0));
            if let Ok(clock_handle) = clock_result.as_mut() {
                clock_handle.start();
                clock = Some(clock_result.unwrap())
            } else {
                warn!(?clock_result, "Could not create clock.")
            }
            let music_track = manager.add_sub_track(TrackBuilder::new()).ok();
            let ambience_track = manager.add_sub_track(TrackBuilder::new()).ok();
            let ui_track = manager.add_sub_track(TrackBuilder::new()).ok();
            if let (Some(music_track), Some(ambience_track), Some(ui_track)) =
                (music_track, ambience_track, ui_track)
            {
                master_tracks.push(music_track); // 0 = music
                master_tracks.push(ambience_track); // 1 = ambience
                master_tracks.push(ui_track); // 2 = ui
            }
            ui_channels.resize_with(num_ui_channels, || {
                UiChannel::new(manager, master_tracks[2].id())
            });
        }

        if let Some(spatial_manager) = spatial_manager.as_mut() {
            let mut scene_result = spatial_manager.add_spatial_scene(SpatialSceneSettings::new());
            if let Ok(scene_handle) = scene_result.as_mut() {
                let listener_result = scene_handle.add_listener(
                    Vec3::<f32>::zero(),
                    Quaternion::<f32>::identity(),
                    ListenerSettings::new(),
                );
                if let Ok(listener_handle) = listener_result {
                    listener = Some(listener_handle)
                } else {
                    warn!(?listener_result, "Could not create listener.")
                }
                for _e in 1..num_sfx_channels {
                    let emitter = scene_handle.add_emitter(
                        Vec3::zero(),
                        EmitterSettings::new()
                            .persist_until_sounds_finish(true)
                            .distances([1.0, 200.0])
                            .attenuation_function(Some(Easing::OutPowf(0.45))),
                    );
                    if let Ok(emitter) = emitter {
                        sfx_channels.push(SfxChannel::new(Some(emitter)))
                    }
                }
                scene = Some(scene_result.unwrap());
            } else {
                warn!(?scene_result, "Could not create scene.")
            }
        }

        Self {
            manager,
            spatial_manager,
            scene,
            clock,
            master_tracks,
            master_effects,
            music_channels: Vec::new(),
            sfx_channels,
            ui_channels,
            ambience_channels: Vec::new(),
            sfx_volume: 1.0,
            ambience_volume: 1.0,
            music_volume: 1.0,
            master_volume: 1.0,
            music_spacing: 1.0,
            listener,
            listener_pos: Vec3::<f32>::zero(),
            listener_ori: Vec3::<f32>::zero(),
            mtm: AssetExt::load_expect("voxygen.audio.music_transition_manifest"),
            subtitles: VecDeque::new(),
            subtitles_enabled: subtitles,
            combat_music_enabled,
        }
    }

    /// Construct in `no-audio` mode for debugging
    pub fn no_audio() -> Self {
        Self {
            manager: None,
            spatial_manager: None,
            scene: None,
            clock: None,
            master_tracks: Vec::new(),
            master_effects: Vec::new(),
            music_channels: Vec::new(),
            sfx_channels: Vec::new(),
            ui_channels: Vec::new(),
            ambience_channels: Vec::new(),
            sfx_volume: 0.0,
            ambience_volume: 0.0,
            music_volume: 0.0,
            master_volume: 0.0,
            music_spacing: 1.0,
            listener: None,
            listener_pos: Vec3::<f32>::zero(),
            listener_ori: Vec3::<f32>::zero(),
            mtm: AssetExt::load_expect("voxygen.audio.music_transition_manifest"),
            subtitles: VecDeque::new(),
            subtitles_enabled: false,
            combat_music_enabled: false,
        }
    }

    /// Drop any unused music channels
    pub fn maintain(&mut self) {
        self.music_channels.retain(|c| !c.is_done());
        self.ambience_channels.retain(|c| !c.is_stopped());
    }

    pub fn get_clock(&mut self) -> Option<&mut ClockHandle> { self.clock.as_mut() }

    pub fn get_clock_time(&self) -> Option<ClockTime> {
        self.clock.as_ref().map(|clock| clock.time())
    }

    /// Play a sound using the AudioManager
    fn play_sound(
        &mut self,
        mut source: StaticSoundData,
        fade_in: Option<f32>,
        delay: Option<f32>,
        spatial: bool,
    ) -> Option<StaticSoundHandle> {
        if let Some(clock) = self.clock.as_mut()
            && (fade_in.is_some() || delay.is_some())
        {
            let fade_in_tween = Tween {
                duration: Duration::from_secs_f32(fade_in.unwrap_or(0.0)),
                ..Default::default()
            };
            source = source
                .fade_in_tween(fade_in_tween)
                .start_time(clock.time() + delay.unwrap_or(0.0) as f64);
        }
        if !spatial {
            if let Some(manager) = self.manager.as_mut()
                && let Ok(s) = manager.play(source)
            {
                Some(s)
            } else {
                None
            }
        } else if let Some(spatial_manager) = self.spatial_manager.as_mut()
            && let Ok(s) = spatial_manager.play(source)
        {
            Some(s)
        } else {
            None
        }
    }

    /// Returns [music channels, ambience channels, sfx channels, ui channels]
    pub fn get_num_active_channels(&self) -> [usize; 4] {
        let mut array = [0, 0, 0, 0] as [usize; 4];
        array[0] = self.music_channels.iter().filter(|c| !c.is_done()).count();
        array[1] = self
            .ambience_channels
            .iter()
            .filter(|c| c.is_active())
            .count();
        array[2] = self.sfx_channels.iter().filter(|c| !c.is_done()).count();
        array[3] = self.ui_channels.iter().filter(|c| !c.is_done()).count();
        array
    }

    /// Retrive an empty sfx channel from the list
    /// This function MUST NOT ever use random values
    fn get_sfx_channel(&mut self) -> Option<&mut SfxChannel> {
        if self.manager.is_some() {
            return self.sfx_channels.iter_mut().find(|c| c.is_done());
        }

        None
    }

    /// Retrive an empty UI channel from the list
    /// This function MUST NOT ever use random values
    fn get_ui_channel(&mut self) -> Option<&mut UiChannel> {
        if self.manager.is_some() {
            if let Some(channel) = self.ui_channels.iter_mut().find(|c| c.is_done()) {
                return Some(channel);
            }
        }

        None
    }

    /// Gets the music channel matching the given tag, of which there should be
    /// only one, if any.
    pub fn get_music_channel(&mut self, channel_tag: MusicChannelTag) -> Option<&mut MusicChannel> {
        if self.manager.is_some() {
            return self
                .music_channels
                .iter_mut()
                .find(|c| c.get_tag() == channel_tag);
        }

        None
    }

    /// Play a music file with the given tag. Pass in the length of the track in
    /// seconds.
    fn play_music(&mut self, sound: &str, channel_tag: MusicChannelTag, length: f32) {
        if self.music_enabled() && self.manager.is_some() {
            let mtm = self.mtm.read();
            if let Some(channel) = self.get_music_channel(channel_tag) {
                let (fade_out, fade_in) = mtm
                    .fade_timings
                    .get(&(channel.get_tag(), channel_tag))
                    .unwrap_or(&(1.0, 0.1));
                let track = channel.get_track();
                let source = load_ogg(sound).output_destination(track.unwrap());
                channel.stop(Some(*fade_out), None);
                channel.set_length(length);
                channel.set_tag(channel_tag);
                channel.set_loop_data(false, LoopPoint::Start, LoopPoint::End);
                let handle = self.play_sound(source, Some(*fade_in), Some(*fade_out), false);
                self.get_music_channel(channel_tag)
                    .map(|channel| channel.set_source(handle));
            } else {
                let current_channel = self.music_channels.last_mut();
                if let Some(current_channel) = current_channel {
                    let (fade_out, _fade_in) = mtm
                        .fade_timings
                        .get(&(current_channel.get_tag(), channel_tag))
                        .unwrap_or(&(1.0, 1.0));
                    current_channel.fade_out(*fade_out, None);
                }
                self.create_music_channel(channel_tag);
                self.play_music(sound, channel_tag, length);
            }
        }
    }

    fn create_music_channel(&mut self, channel_tag: MusicChannelTag) {
        let parent_track = self.master_tracks[0].id();
        if let Some(manager) = self.manager.as_mut() {
            let mut next_music_channel = MusicChannel::new(manager, parent_track);
            next_music_channel.set_volume(1.0);
            next_music_channel.set_tag(channel_tag);
            self.music_channels.push(next_music_channel);
        }
    }

    /// Turn on or off looping
    pub fn set_loop(&mut self, channel_tag: MusicChannelTag, sound_loops: bool) {
        let channel = self.get_music_channel(channel_tag);
        if let Some(channel) = channel {
            let loop_data = channel.get_loop_data();
            channel.set_loop_data(sound_loops, loop_data.1, loop_data.2);
        }
    }

    /// Loops music from start point to end point in seconds
    pub fn set_loop_points(&mut self, channel_tag: MusicChannelTag, start: f32, end: f32) {
        let channel = self.get_music_channel(channel_tag);
        if let Some(channel) = channel {
            channel.set_loop_data(
                true,
                LoopPoint::Point(start as f64),
                LoopPoint::Point(end as f64),
            );
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

    fn build_sfx_sound_data(
        &mut self,
        source: StaticSoundData,
        pos: Vec3<f32>,
    ) -> Option<StaticSoundData> {
        if let Some(scene) = self.scene.as_mut() {
            let emitter = scene.add_emitter(
                pos,
                EmitterSettings::new()
                    .persist_until_sounds_finish(true)
                    .distances([1.0, 200.0])
                    .attenuation_function(Some(Easing::OutPowf(0.45))),
            );
            if let Ok(e) = emitter {
                let sound_data = source.output_destination(&e);
                Some(sound_data)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Set the cutoff of the filter affecting all spatial sfx
    pub fn set_sfx_master_filter(&mut self, frequency: u32) {
        let filter = self
            .master_effects
            .iter_mut()
            .find(|e| matches!(e, MasterEffect::SfxFilter(_)));
        if filter.is_some() {
            let MasterEffect::SfxFilter(f) = filter.unwrap();
            f.set_cutoff(Value::Fixed(frequency as f64), Tween::default());
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
            if self.manager.is_some() && self.sfx_enabled() {
                let sound =
                    load_ogg(sfx_file).volume(Volume::Amplitude(volume.unwrap_or(1.0) as f64));
                if self.listener.is_some()
                    && let Some(channel) = self.get_sfx_channel()
                {
                    channel.set_pos(position);
                    channel.update(position);
                    // We raise volume for spatial sounds because they're way too quiet otherwise
                    let source = self.build_sfx_sound_data(
                        sound.volume(Volume::Amplitude((volume.unwrap_or(1.0) * 5.0) as f64)),
                        position,
                    );
                    if let Some(source) = source {
                        let handle = self.play_sound(source, None, None, true);
                        self.get_sfx_channel().unwrap().set_source(handle);
                    }
                }
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
            if self.manager.is_some() && self.sfx_enabled() {
                let sound =
                    load_ogg(sfx_file).volume(Volume::Amplitude(volume.unwrap_or(1.0) as f64));
                if self.get_ui_channel().is_some() {
                    let handle = self.play_sound(sound, None, None, false);
                    self.get_ui_channel().unwrap().set_source(handle);
                }
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
        if self.manager.is_some() && self.get_ambience_channel(channel_tag).is_some() {
            let channel = self.get_ambience_channel(channel_tag).unwrap();
            if let Some(track) = channel.get_track() {
                let source = load_ogg(sound).loop_region(0.0..).output_destination(track);
                let handle = self.play_sound(source, Some(1.0), None, false);
                self.get_ambience_channel(channel_tag)
                    .unwrap()
                    .set_source(handle);
            }
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
        if self.manager.is_some()
            && self.get_ambience_channel(channel_tag).is_some()
            && trigger_item.is_some()
        {
            let channel = self.get_ambience_channel(channel_tag).unwrap();
            if let Some(track) = channel.get_track() {
                let sound = AudioFrontend::get_sfx_file(trigger_item)
                    .unwrap_or(("", 0.0, Some("")))
                    .0;
                let source = load_ogg(sound)
                    .loop_region(None)
                    .output_destination(track)
                    .volume(Volume::Amplitude(volume.unwrap_or(1.0) as f64));
                channel.set_looping(false);
                channel.fade_to(1.0, 0.0);
                let handle = self.play_sound(source, None, delay, false);
                self.get_ambience_channel(channel_tag)
                    .unwrap()
                    .set_source(handle);
            }
        }
    }

    /// Adds a new ambience channel of the given tag at zero volume
    fn new_ambience_channel(&mut self, channel_tag: AmbienceChannelTag) {
        if self.manager.is_some() {
            let parent_track = self.master_tracks[1].id();
            let ambience_channel = AmbienceChannel::new(
                channel_tag,
                0.0,
                self.manager.as_mut().unwrap(),
                parent_track,
                true,
            );
            self.ambience_channels.push(ambience_channel);
        }
    }

    /// Retrieves the channel currently having the given tag
    /// If no channel with the given tag is found, returns None
    fn get_ambience_channel(
        &mut self,
        channel_tag: AmbienceChannelTag,
    ) -> Option<&mut AmbienceChannel> {
        if self.manager.is_some() {
            self.ambience_channels
                .iter_mut()
                .find(|channel| channel.get_tag() == channel_tag)
        } else {
            None
        }
    }

    pub fn set_listener_pos(&mut self, pos: Vec3<f32>, ori: Vec3<f32>) {
        let tween = Tween {
            duration: Duration::from_secs_f32(0.01),
            ..Default::default()
        };
        self.listener_pos = pos;
        self.listener_ori = ori;
        if let Some(listener) = self.listener.as_mut() {
            let ori_quat = Ori::from(ori).to_quat();
            listener.set_position(pos, tween);
            listener.set_orientation(ori_quat.normalized(), tween);
        }

        for channel in self.sfx_channels.iter_mut() {
            if !channel.is_done() {
                channel.update(pos);
            }
        }
    }

    pub fn get_listener(&mut self) -> Option<&mut ListenerHandle> { self.listener.as_mut() }

    pub fn get_listener_pos(&self) -> Vec3<f32> {
        if self.listener.is_some() {
            self.listener_pos
        } else {
            Vec3::zero()
        }
    }

    pub fn get_listener_ori(&self) -> Vec3<f32> {
        if self.listener.is_some() {
            self.listener_ori
        } else {
            Vec3::zero()
        }
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
    pub fn get_music_volume(&self) -> f32 { self.music_volume }

    /// Retrieves the current setting for ambience volume
    pub fn get_ambience_volume(&self) -> f32 { self.ambience_volume }

    /// Retrieves the current setting for sfx volume
    pub fn get_sfx_volume(&self) -> f32 { self.sfx_volume }

    /// Returns false if volume is 0 or the mute is on
    pub fn music_enabled(&self) -> bool { self.get_music_volume() > 0.0 }

    /// Returns false if volume is 0 or the mute is on
    pub fn ambience_enabled(&self) -> bool { self.get_ambience_volume() > 0.0 }

    /// Returns false if volume is 0 or the mute is on
    pub fn sfx_enabled(&self) -> bool { self.get_sfx_volume() > 0.0 }

    pub fn set_music_volume(&mut self, music_volume: f32) {
        self.music_volume = music_volume;
        let music_volume = self.get_music_volume();

        self.master_tracks[0].set_volume(Volume::Amplitude(music_volume as f64), Tween::default());
    }

    pub fn set_ambience_volume(&mut self, ambience_volume: f32) {
        self.ambience_volume = ambience_volume;
        let ambience_volume = self.get_ambience_volume();

        self.master_tracks[1]
            .set_volume(Volume::Amplitude(ambience_volume as f64), Tween::default());
    }

    /// Sets the volume for both spatial sfx and UI (might separate these
    /// controls later)
    pub fn set_sfx_volume(&mut self, sfx_volume: f32) {
        self.sfx_volume = sfx_volume;
        let sfx_volume = self.get_sfx_volume();

        if let Some(spatial_manager) = self.spatial_manager.as_mut() {
            spatial_manager.main_track().set_volume(
                Volume::Amplitude((sfx_volume * self.master_volume) as f64),
                Tween::default(),
            );
        }

        self.master_tracks[2].set_volume(Volume::Amplitude(sfx_volume as f64), Tween::default());
    }

    pub fn set_music_spacing(&mut self, multiplier: f32) { self.music_spacing = multiplier }

    pub fn set_subtitles(&mut self, enabled: bool) { self.subtitles_enabled = enabled }

    /// Updates volume of the master track
    pub fn set_master_volume(&mut self, master_volume: f32) {
        self.master_volume = master_volume;

        if let Some(manager) = self.manager.as_mut() {
            manager
                .main_track()
                .set_volume(Volume::Amplitude(master_volume as f64), Tween::default());
        }
        // Update spatial manager too
        self.set_sfx_volume(self.get_sfx_volume());
    }

    pub fn stop_all_ambience(&mut self) {
        if self.manager.is_some() {
            for channel in &mut self.ambience_channels {
                channel.stop(None, None);
            }
        }
    }

    pub fn stop_all_music(&mut self) {
        if self.manager.is_some() {
            for channel in &mut self.music_channels {
                channel.stop(None, None);
            }
        }
    }

    pub fn stop_all_sfx(&mut self) {
        if self.manager.is_some() {
            for channel in &mut self.sfx_channels {
                channel.stop();
            }
            for channel in &mut self.ui_channels {
                channel.stop();
            }
        };
    }

    pub fn get_num_music_channels(&self) -> usize { self.music_channels.len() }

    pub fn get_num_ambience_channels(&self) -> usize { self.ambience_channels.len() }
}
