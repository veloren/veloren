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
    pub buffer: String,
    pub start_tick: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub repeat: Option<()>,
    pub positional: Option<Position>,
    pub fading: Option<Fade>,
}

impl Stream {
    pub fn new_music() {}
}

pub struct AudioFrontend {
    device: Device,
    pos: Vec3<f32>,
    ori: Mat4<f32>,
    streams: HashMap<u64, InternalStream>,
    buffers: HashMap<String, Vec<u8>>,
}

struct InternalStream {
    pub sink: SpatialSink, // always use SpatialSink even if no possition is used for now
    pub settings: Stream,
}

impl AudioFrontend {
    pub fn new() -> Self {
        AudioFrontend {
            device: AudioFrontend::get_default_device(),
            pos: Vec3::new(0.0, 0.0, 0.0),
            ori: Mat4::identity(),
            streams: HashMap::<u64, InternalStream>::new(),
            buffers: HashMap::<String, Buffer>::new(),
        }
    }

    /// Returns audio devices that actually work. Filters out the jack audio server for linux.
    pub fn get_audio_devices() -> Vec<Device> {
        rodio::devices()
            .filter(|d| !d.name().contains("jack"))
            .collect()
    }

    /// The rodio::default_output_device will sometimes return an audio device that doesn't actually work.
    /// This function uses get_audio_devices() so that unusable audio devices are filtered out.
    pub fn get_default_device() -> Device {
        AudioFrontend::get_audio_devices()[0]
    }

    pub fn set_audio_device(&mut self, device: Device) {
        self.device = device;
    }

    pub fn set_pos(&self, pos: Vec3<f32>, _vel: Vec3<f32>, ori: Mat4<f32>) {
        self.pos = pos;
        self.ori = ori;
        for (id, int) in self.streams.iter_mut() {
            self.adjust(&int.settings, &mut int.sink);
        }
    }

    /// Returns stream id
    pub fn gen_stream(&self, stream: &Stream) -> u64 {
        static mut id: u64 = 0;
        id += 1;

        let src = self.get_buffer(stream.buffer);
        let mut sink = rodio::SpatialSink::new(
            &self.device,
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
        );
        self.adjust(stream, &mut sink);
        sink.append(src);
        let mut internal = InternalStream {
            sink,
            settings: stream.clone(),
        };
        //p.src.play();
        self.streams.insert(id, internal);

        id
    }

    fn drop_stream(&self, id: u64) {
        self.streams.remove(&id);
    }

    fn adjust(&self, stream: &Stream, sink: &mut SpatialSink) {
        const FALLOFF: f32 = 0.13;
        if let Some(pos) = &stream.positional {
            if pos.relative {
                sink.set_emitter_position([
                    pos.pos.x * FALLOFF,
                    pos.pos.y * FALLOFF,
                    pos.pos.z * FALLOFF,
                ]);
            } else {
                let lpos = self.pos;
                sink.set_emitter_position([
                    (pos.pos.x - lpos.x) * FALLOFF,
                    (pos.pos.y - lpos.y) * FALLOFF,
                    (pos.pos.z - lpos.z) * FALLOFF,
                ]);
            }
            let lori = self.ori;
            //let mut xyz = lori * Vec4::new(pos.pos.x, pos.pos.y, pos.pos.z , 100.5);
            //TODO: FIXME: Wowowowow, thats some ugly code below to get the relative head direction of the camera working.
            // It works on a flat horizontal plane (which will be enought for 90% of people) but we should have someone with a vector math brain look over it...
            let x = lori.into_row_array();
            let mut xy = Vec3::new(x[0] / 0.813, x[1] / 1.3155, 0.0);
            xy.normalize();
            let mut left_ear = Mat3::rotation_z(3.14) * xy;
            let mut right_ear = xy;
            sink.set_left_ear_position(left_ear.into_array());
            sink.set_right_ear_position(right_ear.into_array());
        }
        sink.set_volume(stream.volume);
    }

    fn get_buffer(&self, asset: String) -> Decoder<BufReader<File>> {
        if !self.buffers.contains_key(&asset) {
            let file = assets::load_from_path(&asset).unwrap();
            self.buffers.insert(asset, file);
        }
        rodio::Decoder::new(BufReader::new(self.buffers.get(&asset))).unwrap()
    }
}
