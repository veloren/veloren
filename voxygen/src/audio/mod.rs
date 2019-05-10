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
    thread: JoinHandle<()>,
    sender: Sender<Msg>,
    audio_thread: AudioThread,
}

pub struct AudioThread {
    device: Device,
    pos: Vec3<f32>,
    ori: Mat4<f32>,
    streams: HashMap<u64, InternalStream>, //always use SpatialSink even if no possition is used for now
    buffers: HashMap<u64, Buffer>,
    rec: Receiver<Msg>,
}

struct InternalStream {
    pub sink: SpatialSink,
    pub settings: Stream,
}

impl AudioFrontend {
    pub fn new() -> AudioFrontend {
        let (sender, reciever) = channel();

        let audio_thread = AudioThread::new(reciever);

        let thread = thread::spawn(move || {
            // Start AudioThread
            audio_thread.run();
        });

        AudioFrontend {
            thread,
            sender,
            audio_thread,
        }
    }

    pub fn set_pos(&self, pos: Vec3<f32>, _vel: Vec3<f32>, ori: Mat4<f32>) {
        self.sender.send(Msg::Position(pos, _vel, ori));
    }

    pub fn play_file(&self, data: &str) {
        self.sender.send(Msg::CreateSource(Buffer::File(data)));
    }
}

impl Drop for AudioFrontend {
    fn drop(&mut self) {
        self.sender.send(Msg::Stop);
    }
}

impl AudioThread {
    fn new(rec: Receiver<Msg>) -> Self {
        let device = rodio::default_output_device().unwrap();
        AudioThread {
            device,
            pos: Vec3::new(0.0, 0.0, 0.0),
            ori: Mat4::identity(),
            streams: HashMap::new(),
            buffers: HashMap::new(),
            rec,
        }
    }

    fn run(&self) {
        loop {
            match self.rec.try_recv() {
                Ok(msg) => match msg {
                    Msg::Stop => break,
                    Msg::Position(pos, vel, ori) => self.set_pos(pos, vel, ori),
                    Msg::CreateSource(data) => self.gen_stream(data),
                },
                Err(err) => match err {
                    TryRecvError::Empty => (),
                    TryRecvError::Disconnected => break,
                },
            }
        }
    }

    pub fn set_pos(&self, pos: Vec3<f32>, _vel: Vec3<f32>, ori: Mat4<f32>) {
        for (id, int) in self.streams.iter_mut() {
            self.adjust(&int.settings, &mut int.sink);
        }
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

    fn play(&self, data: Buffer) {
        let src = assets::load(&data).expect("Could not load music");
        let mut sink = rodio::SpatialSink::new(
            &self.device,
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
        );
        self.adjust(stream, &mut sink);
        sink.append(src);
    }

    /*
    fn gen_stream(&self, buffer: &Buffer) {
        let stream = Stream {
            buffer: u64,
            start_tick: Duration,
            duration: Duration,
            volume: 1.0,
        }
        if let Some(buffer) = self.buffers.get(stream.buffer) {
            let src = self.create_source(buffer);
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
        }
    }

    fn gen_buffer(&self, id: u64, buffer: &Buffer) {
        // debug!("generate buffer: {:?}", buffer);
        self.buffers.insert(id, buffer.clone());
    }

    fn create_source(&self, buffer: &Buffer) -> Decoder<BufReader<File>> {
        match buffer {
            Buffer::File(file) => {
                let file = assets::load(&file).expect("Could not load music");
                rodio::Decoder::new(BufReader::new(file)).unwrap()
            }
            Buffer::Raw(..) => {
                panic!("raw buffers not implemented yet");
            }
        }
    }
    */
}
