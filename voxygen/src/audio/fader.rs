#[derive(PartialEq, Clone, Copy)]
pub struct Fader {
    length: f32,
    running_time: f32,
    volume_from: f32,
    volume_to: f32,
    is_running: bool,
}

fn lerp(t: f32, a: f32, b: f32) -> f32 { (1.0 - t) * a + t * b }

impl Fader {
    pub fn fade(time: f32, volume_from: f32, volume_to: f32) -> Self {
        Self {
            length: time,
            running_time: 0.0,
            volume_from,
            volume_to,
            is_running: true,
        }
    }

    pub fn fade_in(time: f32) -> Self {
        Self {
            length: time,
            running_time: 0.0,
            volume_from: 0.0,
            volume_to: 1.0,
            is_running: true,
        }
    }

    pub fn fade_out(time: f32, volume_from: f32) -> Self {
        Self {
            length: time,
            running_time: 0.0,
            volume_from,
            volume_to: 0.0,
            is_running: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.is_running {
            self.running_time = self.running_time + dt;
            if self.running_time >= self.length {
                self.running_time = self.length;
                self.is_running = false;
            }
        }
    }

    pub fn get_volume(&self) -> f32 {
        lerp(
            self.running_time / self.length,
            self.volume_from,
            self.volume_to,
        )
    }

    pub fn is_finished(&self) -> bool { self.running_time >= self.length || !self.is_running }
}
