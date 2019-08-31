use common::assets;
use rodio::{Decoder, Device, Sink, SpatialSink};

const LEFT_EAR : [f32; 3] = [1.0, 0.0, 0.0];
const RIGHT_EAR : [f32; 3] = [-1.0, 0.0, 0.0];

#[derive(PartialEq)]
enum AudioType {
    Sfx,
    Music,
}

struct Channel {
    id: usize,
    sink: SpatialSink,
    audio_type: AudioType,
}

pub struct AudioFrontend {
    pub device: String,
    pub device_list: Vec<String>,
    audio_device: Option<Device>,

    channels: Vec<Channel>,
    next_channel_id: usize,

    sfx_volume: f32,
    music_volume: f32,
}

impl AudioFrontend {
    /// Construct with given device
    pub fn new(device: String) -> Self {
        Self {
            device: device.clone(),
            device_list: list_devices(),
            audio_device: get_device_raw(device),
            channels: Vec::new(),
            next_channel_id: 0,
            sfx_volume: 1.0,
            music_volume: 1.0,
        }
    }

    /// Construct in `no-audio` mode for debugging
    pub fn no_audio() -> Self {
        Self {
            device: "none".to_string(),
            device_list: list_devices(),
            audio_device: None,
            channels: Vec::new(),
            next_channel_id: 0,
            sfx_volume: 1.0,
            music_volume: 1.0,
        }
    }

    /// Maintain audio
    pub fn maintain(&mut self) {
        let mut stopped_channels = Vec::<usize>::new();
        for (i, channel) in self.channels.iter().enumerate() {
            if channel.sink.empty() {
                stopped_channels.push(i);
            }
        }
        for i in stopped_channels.iter().rev() {
            self.channels.remove(*i);
        }
    }

    /// Play specfied sound file.
    ///```ignore
    ///audio.play_sound("voxygen.audio.sfx.step");
    ///```
    pub fn play_sound(&mut self, sound: String) -> usize {
        let id = self.next_channel_id;
        self.next_channel_id += 1;

        if let Some(device) = &self.audio_device {
            let sink = SpatialSink::new(device, [0.0, 0.0, 0.0], LEFT_EAR, RIGHT_EAR);

            let file = assets::load_file(&sound, &["wav", "ogg"]).unwrap();
            let sound = rodio::Decoder::new(file).unwrap();

            sink.append(sound);

            self.channels.push(Channel {
                id,
                sink,
                audio_type: AudioType::Music,
            });
        }

        id
    }

    pub fn get_sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    pub fn get_music_volume(&self) -> f32 {
        self.music_volume
    }

    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume;

        for channel in self.channels.iter() {
            if channel.audio_type == AudioType::Sfx {
                channel.sink.set_volume(volume);
            }
        }
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume;

        for channel in self.channels.iter() {
            if channel.audio_type == AudioType::Music {
                channel.sink.set_volume(volume);
            }
        }
    }

    pub fn set_device(&mut self, name: String) {
        self.device = name.clone();
        self.audio_device = get_device_raw(name);
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

fn get_device_raw(device: String) -> Option<Device> {
    rodio::output_devices().find(|d| d.name() == device)
}
