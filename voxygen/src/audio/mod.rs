pub mod channel;
pub mod fader;
pub mod soundcache;
use channel::{AudioType, Channel};
use fader::Fader;
use soundcache::SoundCache;

use common::assets;
use rodio::{Decoder, Device, SpatialSink};
use vek::*;

const FALLOFF: f32 = 0.13;

pub struct AudioFrontend {
    pub device: String,
    pub device_list: Vec<String>,
    audio_device: Option<Device>,
    sound_cache: SoundCache,

    channels: Vec<Channel>,
    next_channel_id: usize,

    sfx_volume: f32,
    music_volume: f32,

    listener_pos: Vec3<f32>,
    listener_ori: Vec3<f32>,

    listener_pos_left: [f32; 3],
    listener_pos_right: [f32; 3],
}

impl AudioFrontend {
    /// Construct with given device
    pub fn new(device: String, channel_num: usize) -> Self {
        let mut channels = Vec::with_capacity(channel_num);
        let audio_device = get_device_raw(&device);
        if let Some(audio_device) = &audio_device {
            for i in (0..channel_num) {
                channels.push(Channel::new(&audio_device));
            }
        }
        Self {
            device: device.clone(),
            device_list: list_devices(),
            audio_device,
            sound_cache: SoundCache::new(),
            channels: channels,
            next_channel_id: 1,
            sfx_volume: 1.0,
            music_volume: 1.0,
            listener_pos: Vec3::zero(),
            listener_ori: Vec3::zero(),
            listener_pos_left: [0.0; 3],
            listener_pos_right: [0.0; 3],
        }
    }

    /// Construct in `no-audio` mode for debugging
    pub fn no_audio() -> Self {
        Self {
            device: "none".to_string(),
            device_list: list_devices(),
            audio_device: None,
            sound_cache: SoundCache::new(),
            channels: Vec::new(),
            next_channel_id: 1,
            sfx_volume: 1.0,
            music_volume: 1.0,
            listener_pos: Vec3::zero(),
            listener_ori: Vec3::zero(),
            listener_pos_left: [0.0; 3],
            listener_pos_right: [0.0; 3],
        }
    }

    /// Maintain audio
    pub fn maintain(&mut self, dt: f32) {
        for (i, channel) in self.channels.iter_mut().enumerate() {
            channel.update(dt);
        }
    }

    pub fn get_channel(&mut self) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|c| c.is_done())
    }

    /// Play specfied sound file.
    ///```ignore
    ///audio.play_sound("voxygen.audio.sfx.step");
    ///```
    pub fn play_sound(&mut self, sound: String, pos: Vec3<f32>) -> usize {
        let id = self.next_channel_id;
        self.next_channel_id += 1;

        if let Some(device) = &self.audio_device {
            let calc_pos = [
                (pos.x - self.listener_pos.x) * FALLOFF,
                (pos.y - self.listener_pos.y) * FALLOFF,
                (pos.z - self.listener_pos.z) * FALLOFF,
            ];

            let sound = self.sound_cache.load_sound(sound);

            let left_ear = self.listener_pos_left;
            let right_ear = self.listener_pos_right;

            if let Some(channel) = self.get_channel() {
                channel.set_id(id);
                channel.set_emitter_position(calc_pos);
                channel.set_left_ear_position(left_ear);
                channel.set_right_ear_position(right_ear);
                channel.play(sound);
            } else {
                println!("No available channels!");
            }
        }

        id
    }

    pub fn set_listener_pos(&mut self, pos: &Vec3<f32>, ori: &Vec3<f32>) {
        self.listener_pos = pos.clone();
        self.listener_ori = ori.normalized();

        let up = Vec3::new(0.0, 0.0, 1.0);

        let pos_left = up.cross(self.listener_ori.clone()).normalized();
        let pos_right = self.listener_ori.cross(up.clone()).normalized();

        self.listener_pos_left = pos_left.into_array();
        self.listener_pos_right = pos_right.into_array();

        for channel in self.channels.iter_mut() {
            if channel.get_audio_type() == AudioType::Sfx {
                channel.set_emitter_position([
                    (channel.pos.x - self.listener_pos.x) * FALLOFF,
                    (channel.pos.y - self.listener_pos.y) * FALLOFF,
                    (channel.pos.z - self.listener_pos.z) * FALLOFF,
                ]);
                channel.set_left_ear_position(pos_left.into_array());
                channel.set_right_ear_position(pos_right.into_array());
            }
        }
    }

    pub fn play_music(&mut self, sound: String) -> usize {
        let id = self.next_channel_id;
        self.next_channel_id += 1;

        if let Some(device) = &self.audio_device {
            let file = assets::load_file(&sound, &["ogg"]).unwrap();
            let sound = Decoder::new(file).unwrap();

            if let Some(channel) = self.get_channel() {
                channel.set_id(id);
                channel.play(sound);
            }
        }

        id
    }

    pub fn stop_channel(&mut self, channel_id: usize, fader: Fader) {
        let index = self.channels.iter().position(|c| c.get_id() == channel_id);
        if let Some(index) = index {
            self.channels[index].stop(fader);
        }
    }

    pub fn get_sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    pub fn get_music_volume(&self) -> f32 {
        self.music_volume
    }

    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume;

        for channel in self.channels.iter_mut() {
            if channel.get_audio_type() == AudioType::Sfx {
                channel.set_volume(volume);
            }
        }
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume;

        for channel in self.channels.iter_mut() {
            if channel.get_audio_type() == AudioType::Music {
                channel.set_volume(volume);
            }
        }
    }

    // TODO: figure out how badly this will break things when it is called
    pub fn set_device(&mut self, name: String) {
        self.device = name.clone();
        self.audio_device = get_device_raw(&name);
    }
}

pub fn select_random_music() -> String {
    let soundtracks = load_soundtracks();
    let index = rand::random::<usize>() % soundtracks.len();
    soundtracks[index].clone()
}

/// Returns the default audio device.
/// Does not return rodio Device struct in case our audio backend changes.
pub fn get_default_device() -> String {
    rodio::default_output_device()
        .expect("No audio output devices detected.")
        .name()
}

/// Load the audio file directory selected by genre.
pub fn load_soundtracks() -> Vec<String> {
    let assets = assets::read_dir("voxygen.audio.soundtrack").unwrap();
    let soundtracks = assets
        .filter_map(|entry| {
            entry.ok().map(|f| {
                let path = f.path();
                path.to_string_lossy().into_owned()
            })
        })
        .collect::<Vec<String>>();

    soundtracks
}

/// Returns a vec of the audio devices available.
/// Does not return rodio Device struct in case our audio backend changes.
pub fn list_devices() -> Vec<String> {
    list_devices_raw().iter().map(|x| x.name()).collect()
}

/// Returns vec of devices
fn list_devices_raw() -> Vec<Device> {
    rodio::output_devices().collect()
}

fn get_device_raw(device: &str) -> Option<Device> {
    rodio::output_devices().find(|d| d.name() == device)
}
