use common::assets;
use rand::prelude::*;
use rodio::{Decoder, Device, Source, SpatialSink};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread,
    thread::{sleep, JoinHandle},
    time::Duration,
};
use vek::*;

pub struct AudioFrontend {
    device: Device,
    // streams: HashMap<String, SpatialSink>, //always use SpatialSink even if no possition is used for now
    stream: SpatialSink,
}

impl AudioFrontend {
    pub fn new() -> Self {
        let mut device = rodio::default_output_device().unwrap();

        for d in rodio::devices() {
            if d.name().contains("jack") {
                continue;
            }

            device = d;
            break;
        }

        let mut sink =
            rodio::SpatialSink::new(&device, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);

        AudioFrontend {
            device,
            // streams: HashMap::<String, SpatialSink>::new(),
            stream: sink,
        }
    }

    pub fn play_music(&mut self, path: &str) {
        let bufreader = assets::load_from_path(path).unwrap();
        let src = Decoder::new(bufreader).unwrap();

        let mut sink = rodio::SpatialSink::new(
            &self.device,
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
        );

        sink.append(src);

        // self.streams.insert(path.to_string(), sink);
        self.stream = sink;
    }

    pub fn maintain(&mut self) {
        let music = [
            "voxygen/audio/soundtrack/Ethereal_Bonds.ogg",
            "voxygen/audio/soundtrack/Field_Grazing.mp3",
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
}
