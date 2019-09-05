use crate::audio::fader::Fader;
use rodio::{Decoder, Device, Sample, Source, SpatialSink};
use std::fs::File;
use std::io::BufReader;
use vek::*;

#[derive(PartialEq, Clone, Copy)]
pub enum AudioType {
    Sfx,
    Music,
    None,
}

#[derive(PartialEq, Clone, Copy)]
enum ChannelState {
    Init,
    ToPlay,
    Loading,
    Playing,
    Stopping,
    Stopped,
}

pub struct Channel {
    id: usize,
    sink: SpatialSink,
    audio_type: AudioType,
    state: ChannelState,
    fader: Fader,
    pub pos: Vec3<f32>,
}

// TODO: Implement asynchronous loading
impl Channel {
    /// Create an empty channel for future use
    pub fn new(device: &Device) -> Self {
        Self {
            id: 0,
            sink: SpatialSink::new(device, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
            audio_type: AudioType::None,
            state: ChannelState::Stopped,
            fader: Fader::fade_in(0.0),
            pos: Vec3::zero(),
        }
    }

    pub fn music(id: usize, device: &Device, bufr: BufReader<File>) -> Self {
        let sink = SpatialSink::new(device, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
        let sound = Decoder::new(bufr).unwrap();

        sink.append(sound);

        Self {
            id,
            sink,
            audio_type: AudioType::Music,
            state: ChannelState::Playing,
            fader: Fader::fade_in(0.0),
            pos: Vec3::zero(),
        }
    }

    pub fn sfx(id: usize, sink: SpatialSink, pos: Vec3<f32>) -> Self {
        Self {
            id,
            sink,
            audio_type: AudioType::Sfx,
            state: ChannelState::Playing,
            fader: Fader::fade_in(0.0),
            pos,
        }
    }

    pub fn play<S>(&mut self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
        <S as std::iter::Iterator>::Item: std::fmt::Debug,
    {
        self.state = ChannelState::Playing;
        self.sink.append(source);
    }

    pub fn is_done(&self) -> bool {
        self.sink.empty() || self.state == ChannelState::Stopped
    }

    pub fn stop(&mut self, fader: Fader) {
        self.state = ChannelState::Stopping;
        self.fader = fader;
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn set_id(&mut self, new_id: usize) {
        self.id = new_id;
    }

    pub fn get_audio_type(&self) -> AudioType {
        self.audio_type
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.sink.set_volume(volume);
    }

    pub fn set_emitter_position(&mut self, pos: [f32; 3]) {
        self.sink.set_emitter_position(pos);
    }

    pub fn set_left_ear_position(&mut self, pos: [f32; 3]) {
        self.sink.set_left_ear_position(pos);
    }

    pub fn set_right_ear_position(&mut self, pos: [f32; 3]) {
        self.sink.set_right_ear_position(pos);
    }

    pub fn update(&mut self, dt: f32) {
        match self.state {
            ChannelState::Init | ChannelState::ToPlay | ChannelState::Loading => {}
            ChannelState::Playing => {}
            ChannelState::Stopping => {
                self.fader.update(dt);
                self.sink.set_volume(self.fader.get_volume());

                if self.fader.is_finished() {
                    self.state = ChannelState::Stopped;
                }
            }
            ChannelState::Stopped => {}
        }
    }
}
