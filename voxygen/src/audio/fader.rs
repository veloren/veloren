//! Controls volume transitions for Audio Channels

use std::time::Duration;

/// Faders are attached to channels with initial and target volumes as well as a
/// transition time.
#[derive(PartialEq, Clone, Copy)]
pub struct Fader {
    length: Duration,
    running_time: Duration,
    volume_from: f32,
    volume_to: f32,
    is_running: bool,
}
/// Enables quick lookup of whether a fader is increasing or decreasing the
/// channel volume
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FadeDirection {
    In,
    Out,
}

fn lerp(t: f32, a: f32, b: f32) -> f32 { (1.0 - t) * a + t * b }

impl Fader {
    pub fn fade(length: Duration, volume_from: f32, volume_to: f32) -> Self {
        Self {
            length,
            running_time: Duration::default(),
            volume_from,
            volume_to,
            is_running: true,
        }
    }

    pub fn fade_in(time: Duration, volume_to: f32) -> Self { Self::fade(time, 0.0, volume_to) }

    pub fn fade_out(time: Duration, volume_from: f32) -> Self { Self::fade(time, volume_from, 0.0) }

    /// Used to update the `target` volume of the fader when the max or min
    /// volume changes. This occurs when the player changes their in-game
    /// volume setting during a fade. Updating the target in this case prevents
    /// the final fade volume from falling outside of the newly configured
    /// volume range.
    pub fn update_target_volume(&mut self, volume: f32) {
        match self.direction() {
            FadeDirection::In => {
                self.volume_to = volume;
            },
            FadeDirection::Out => {
                if self.get_volume() > volume {
                    self.volume_from = volume;
                }
            },
        }
    }

    pub fn direction(&self) -> FadeDirection {
        if self.volume_to < self.volume_from {
            FadeDirection::Out
        } else {
            FadeDirection::In
        }
    }

    /// Called each tick to update the volume and state
    pub fn update(&mut self, dt: Duration) {
        if self.is_running {
            self.running_time += dt;
            if self.running_time >= self.length {
                self.running_time = self.length;
                self.is_running = false;
            }
        }
    }

    pub fn get_volume(&self) -> f32 {
        lerp(
            self.running_time.as_nanos() as f32 / self.length.as_nanos() as f32,
            self.volume_from,
            self.volume_to,
        )
    }

    pub fn is_finished(&self) -> bool { self.running_time >= self.length || !self.is_running }
}

/// Returns a stopped fader with no running duration
impl Default for Fader {
    fn default() -> Self {
        Self {
            length: Duration::default(),
            running_time: Duration::default(),
            volume_from: 0.0,
            volume_to: 1.0,
            is_running: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_direction_in() {
        let fader = Fader::fade_in(Duration::from_secs(10), 0.0);

        assert_eq!(fader.direction(), FadeDirection::In);
    }

    #[test]
    fn fade_direction_out() {
        let fader = Fader::fade_out(Duration::from_secs(10), 1.0);

        assert_eq!(fader.direction(), FadeDirection::Out);
    }

    #[test]
    fn fade_out_completes() {
        let mut fader = Fader::fade_out(Duration::from_secs(10), 1.0);

        // Run for the full duration
        fader.update(Duration::from_secs(10));

        assert_eq!(fader.get_volume(), 0.0);
        assert!(fader.is_finished());
    }

    #[test]
    fn update_target_volume_fading_out_when_currently_above() {
        let mut fader = Fader::fade_out(Duration::from_secs(20), 1.0);

        // After 0.1s, the fader should still be close to 1.0
        fader.update(Duration::from_millis(100));

        // Reduce volume to 0.4. We are currently above that.
        fader.update_target_volume(0.4);

        // The volume should immediately reduce to < 0.4 on the next update
        fader.update(Duration::from_millis(100));

        assert!(fader.get_volume() < 0.4)
    }

    #[test]
    fn update_target_volume_fading_out_when_currently_below() {
        let mut fader = Fader::fade_out(Duration::from_secs(10), 0.8);

        // After 9s, the fader should be close to 0
        fader.update(Duration::from_secs(9));

        // Notify of a volume increase to 1.0. We are already far below that.
        fader.update_target_volume(1.0);

        // The fader should be unaffected by the new value, and continue dropping
        fader.update(Duration::from_millis(100));

        assert!(fader.get_volume() < 0.2);
    }

    #[test]
    fn update_target_volume_fading_in_when_currently_above() {
        let mut fader = Fader::fade_in(Duration::from_secs(10), 1.0);

        // After 9s, the fader should be close to 1.0
        fader.update(Duration::from_secs(9));

        // Reduce volume to 0.4. We are currently above that.
        fader.update_target_volume(0.4);

        // Run out the fader. It's volume should be 0.4
        fader.update(Duration::from_secs(1));

        assert_eq!(fader.get_volume(), 0.4);
    }

    #[test]
    fn update_target_volume_fading_in_when_currently_below() {
        let mut fader = Fader::fade_in(Duration::from_secs(20), 1.0);

        // After 0.1s, the fader should still be close to 0.0
        fader.update(Duration::from_millis(100));

        // Reduce volume to 0.4. The volume_to should be reduced accordingly.
        fader.update_target_volume(0.4);

        assert_eq!(fader.volume_to, 0.4);
    }
}
