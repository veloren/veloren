use crate::span;
use std::{
    thread,
    time::{Duration, Instant},
};

const CLOCK_SMOOTHING: f64 = 0.9;

pub struct Clock {
    last_sys_time: Instant,
    last_delta: Option<Duration>,
    running_average_delta: f64,
    compensation: f64,
}

impl Clock {
    pub fn start() -> Self {
        Self {
            last_sys_time: Instant::now(),
            last_delta: None,
            running_average_delta: 0.0,
            compensation: 1.0,
        }
    }

    pub fn get_tps(&self) -> f64 { 1.0 / self.running_average_delta }

    pub fn get_last_delta(&self) -> Duration {
        self.last_delta.unwrap_or_else(|| Duration::new(0, 0))
    }

    pub fn get_avg_delta(&self) -> Duration { Duration::from_secs_f64(self.running_average_delta) }

    pub fn tick(&mut self, tgt: Duration) {
        span!(_guard, "tick", "Clock::tick");
        let delta = Instant::now().duration_since(self.last_sys_time);

        // Attempt to sleep to fill the gap.
        if let Some(sleep_dur) = tgt.checked_sub(delta) {
            if self.running_average_delta != 0.0 {
                self.compensation =
                    (self.compensation + (tgt.as_secs_f64() / self.running_average_delta) - 1.0)
                        .max(0.0)
            }

            let sleep_secs = sleep_dur.as_secs_f64() * self.compensation;
            if sleep_secs > 0.0 {
                thread::sleep(Duration::from_secs_f64(sleep_secs));
            }
        }

        let delta = Instant::now().duration_since(self.last_sys_time);

        self.last_sys_time = Instant::now();
        self.last_delta = Some(delta);
        self.running_average_delta = if self.running_average_delta == 0.0 {
            delta.as_secs_f64()
        } else {
            CLOCK_SMOOTHING * self.running_average_delta
                + (1.0 - CLOCK_SMOOTHING) * delta.as_secs_f64()
        };
    }
}
