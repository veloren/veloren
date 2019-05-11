//extern crate byteorder;
extern crate lewton;
extern crate rodio;

use common::assets;
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

#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub relative: bool,
    pub pos: Vec3<f32>,
    pub vel: Vec3<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fade {
    pub in_duration: Duration,
    pub out_duration: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stream {
    pub buffer: u64,
    pub start_tick: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub repeat: Option<()>,
    pub positional: Option<Position>,
    pub fading: Option<Fade>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Buffer {
    File(PathBuf),
    Raw(Vec<u8>),
}

enum Msg {
    Position(Vec3<f32>, Vec3<f32>, Mat4<f32>),
    CreateSource(Buffer),
    Stop,
}

pub struct AudioFrontend {
    device: Device,
    streams: HashMap<String, SpatialSink>, //always use SpatialSink even if no possition is used for now
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

        AudioFrontend {
            device,
            streams: HashMap::<String, SpatialSink>::new(),
        }
    }

    pub fn play_music(&mut self, path: &str) {
        let file = File::open(format!("assets/{}", path)).unwrap();
        let src = Decoder::new(BufReader::new(file)).unwrap();

        let mut sink = rodio::SpatialSink::new(
            &self.device,
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
        );

        sink.append(src);

        self.streams.insert(path.to_string(), sink);
    }
}
