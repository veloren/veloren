use rodio::SpatialSink;
use crate::audio::fader::Fader;

#[derive(PartialEq, Clone, Copy)]
pub enum AudioType {
    Sfx,
    Music,
}

#[derive(PartialEq, Clone, Copy)]
enum ChannelState {
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
}

impl Channel {
    pub fn music(id: usize, sink: SpatialSink) -> Self {
        Self {
            id,
            sink,
            audio_type: AudioType::Music,
            state: ChannelState::Playing,
            fader: Fader::fade_in(0.0),
        }
    }

    pub fn sfx(id: usize, sink: SpatialSink) -> Self {
        Self {
            id,
            sink,
            audio_type: AudioType::Sfx,
            state: ChannelState::Playing,
            fader: Fader::fade_in(0.0),
        }
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

    pub fn get_audio_type(&self) -> AudioType {
        self.audio_type
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.sink.set_volume(volume);
    }

    pub fn set_left_ear_position(&mut self, pos: [f32; 3]) {
        self.sink.set_left_ear_position(pos);
    }

    pub fn set_right_ear_position(&mut self, pos: [f32; 3]) {
        self.sink.set_right_ear_position(pos);
    }

    pub fn update(&mut self, dt: f32) {
        match self.state {
            ChannelState::Playing => {},
            ChannelState::Stopping  => {
                self.fader.update(dt);
                self.sink.set_volume(self.fader.get_volume());
                if self.fader.is_finished() {
                    self.state = ChannelState::Stopped;
                }
            },
            ChannelState::Stopped => {},
        }
    }
}
