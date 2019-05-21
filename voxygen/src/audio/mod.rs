use crate::settings::AudioSettings;
use common::assets;
use rand::prelude::*;
use rodio::{Decoder, Device, Source, SpatialSink};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    iter::{Filter, Iterator},
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread,
    thread::{sleep, JoinHandle},
    time::Duration,
};
use vek::*;

pub struct AudioFrontend {
    device: Device,
    // Performance optimisation, iterating through available audio devices takes time
    devices: Vec<Device>,
    // streams: HashMap<String, SpatialSink>, //always use SpatialSink even if no possition is used for now
    stream: SpatialSink,
}

impl AudioFrontend {
    pub fn new(settings: &AudioSettings) -> Self {
        let mut device = rodio::output_devices()
            .find(|x| x.name() == settings.audio_device)
            .or_else(rodio::default_output_device)
            .expect("No Audio devices found");

        let mut sink =
            rodio::SpatialSink::new(&device, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
        sink.set_volume(settings.music_volume);

        AudioFrontend {
            device,
            // streams: HashMap::<String, SpatialSink>::new(),
            stream: sink,
            devices: AudioFrontend::get_devices_raw(),
        }
    }

    pub fn play_music(&mut self, path: &str) {
        let bufreader = assets::load_from_path(path).unwrap();
        let src = Decoder::new(bufreader).unwrap();

        // TODO: stop previous audio from playing. Sink has this ability, but
        // SpatialSink does not for some reason. This means that we will
        // probably want to use sinks for music, and SpatialSink for sfx.
        self.stream.append(src);
    }

    pub fn maintain(&mut self) {
        let music = [
            "voxygen/audio/soundtrack/Ethereal_Bonds.ogg",
            "voxygen/audio/soundtrack/field_grazing.ogg",
            "voxygen/audio/soundtrack/fiesta_del_pueblo.ogg",
            "voxygen/audio/soundtrack/library_theme_with_harpsichord.ogg",
            "voxygen/audio/soundtrack/Mineral_Deposits.ogg",
            "voxygen/audio/soundtrack/Ruination.ogg",
            "voxygen/audio/soundtrack/sacred_temple.ogg",
            "voxygen/audio/soundtrack/Snowtop.ogg",
            "voxygen/audio/soundtrack/veloren_title_tune-3.ogg",
        ];
        if self.stream.empty() {
            let i = rand::random::<usize>() % music.len();
            self.play_music(music[i])
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.stream.set_volume(volume.min(1.0).max(0.0))
    }

    /// Returns a vec of the audio devices available.
    /// Does not return rodio Device struct in case our audio backend changes.
    pub fn get_devices(&self) -> Vec<String> {
        self.devices.iter().map(|x| x.name()).collect()
    }

    /// Returns vec of devices
    fn get_devices_raw() -> Vec<Device> {
        rodio::output_devices().collect()
    }

    /// Caches vec of devices for later reference
    fn collect_devices(&mut self) {
        self.devices = AudioFrontend::get_devices_raw()
    }

    /// Returns the default audio device.
    /// Does not return rodio Device struct in case our audio backend changes.
    pub fn get_default_device() -> String {
        rodio::default_output_device()
            .expect("No audio output devices detected.")
            .name()
    }

    /// Returns the name of the current audio device.
    /// Does not return rodio Device struct in case our audio backend changes.
    pub fn get_device(&self) -> String {
        self.device.name()
    }

    /// Sets the current audio device from a string.
    /// Does not use the rodio Device struct in case that detail changes.
    /// If the string is an invalid audio device, then no change is made.
    pub fn set_device(&mut self, name: String) {
        if let Some(dev) = rodio::output_devices().find(|x| x.name() == name) {
            self.device = dev;
            self.stream = rodio::SpatialSink::new(
                &self.device,
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
            );
        }
    }
}
