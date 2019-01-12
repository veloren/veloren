// Standard
use std::{
    thread,
    time::{Duration, SystemTime},
};

const CLOCK_SMOOTHING: f64 = 0.9;

pub struct Clock {
    last_sys_time: SystemTime,
    last_delta: Option<Duration>,
    running_tps_average: f64,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            last_sys_time: SystemTime::now(),
            last_delta: None,
            running_tps_average: 0.0,
        }
    }

    pub fn get_tps(&self) -> f64 { self.running_tps_average }

    pub fn get_last_delta(&self) -> Duration { self.last_delta.unwrap_or(Duration::new(0, 0)) }

    pub fn get_avg_delta(&self) -> Duration { Duration::from_float_secs(self.running_tps_average) }

    pub fn tick(&mut self, tgt: Duration) {
        let delta = SystemTime::now()
            .duration_since(self.last_sys_time)
            .expect("Time went backwards!");

        // Attempt to sleep to fill the gap
        if let Some(sleep_dur) = tgt.checked_sub(delta) {
            thread::sleep(sleep_dur);
        }

        let delta = SystemTime::now()
            .duration_since(self.last_sys_time)
            .expect("Time went backwards!");

        self.last_sys_time = SystemTime::now();
        self.last_delta = Some(delta);
        self.running_tps_average =
            CLOCK_SMOOTHING * self.running_tps_average +
            (1.0 - CLOCK_SMOOTHING) * delta.as_float_secs();
    }
}
