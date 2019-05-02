use common::audio::{audio_gen::AudioGen, Buffer, Stream};
use rodio::{Decoder, Device, Source, SpatialSink};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    sync::mpsc::{channel, Reciever, Sender, TryRecvError},
    thread::sleep,
    time::Duration,
};
use std::{thread, thread::JoinHandle};
use vek::*;
use assets;

enum Msg {
    Position(Vec3<f32>, Mat4(f32)),
    Play(&str),
    Stop
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
}

struct InternalStream {
    pub sink: SpatialSink,
    pub settings: Stream,
}

impl AudioFrontend {
    pub fn new() -> Manager<AudioFrontend> {
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
        self.sender.send(Position(pos, vel, ori));
    }

    pub fn play(&self, data: &str>) {
        self.sender.send(Play(data));
    }
}

impl Drop for AudioThread {
    fn drop(&mut self) {
        self.sender.send(Msg::Stop);
    }
}

impl AudioThread {
    fn new(rec: Reciever) -> Self {
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
            match rec.try_recv() {
                Ok(msg) => match msg {
                    Stop => break,
                    Position(pos, vel, ori) => self.set_pos(pos, vel, ori),
                    Play(data) => self.play(data),
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

    fn play(data: &str) {
        let src = assets::load(data).expect("Could not load music");
        let mut sink = rodio::SpatialSink::new(
            &self.device,
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
        );
        self.adjust(stream, &mut sink);
        sink.append(src);
    }
}
