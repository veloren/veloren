use std::{
    thread,
    time::{Duration, Instant},
};

const CLOCK_SMOOTHING: f64 = 0.9;

// TODO: When duration_float is stable, replace `as_millis()` with `as_secs_f64()` or
// something similar
pub struct Clock {
    last_sys_time: Instant,
    last_delta: Option<Duration>,
    running_tps_average: Duration,
    compensation: f64,
}

impl Clock {
    pub fn start() -> Self {
        Self {
            last_sys_time: Instant::now(),
            last_delta: None,
            running_tps_average: Duration::default(),
            compensation: 1.0,
        }
    }

    pub fn get_tps(&self) -> f64 {
        1.0 / (self.running_tps_average.as_millis() as f64 / 1000.0)
    }

    pub fn get_last_delta(&self) -> Duration {
        self.last_delta.unwrap_or_else(|| Duration::new(0, 0))
    }

    pub fn get_avg_delta(&self) -> Duration {
        self.running_tps_average
    }

    pub fn tick(&mut self, tgt: Duration) {
        let delta = Instant::now().duration_since(self.last_sys_time);

        // Attempt to sleep to fill the gap.
        if let Some(sleep_dur) = tgt.checked_sub(delta) {
            if self.running_tps_average != Duration::default() {
                self.compensation = (self.compensation
                    + tgt.as_millis() as f64 / self.running_tps_average.as_millis() as f64
                    - 1.0)
                    .max(0.0);
            }

            let sleep = (sleep_dur.as_millis() as f64 * self.compensation) as u64;
            if sleep > 0 {
                thread::sleep(Duration::from_millis(sleep));
            }
        }

        let delta = Instant::now().duration_since(self.last_sys_time);

        self.last_sys_time = Instant::now();
        self.last_delta = Some(delta);
        self.running_tps_average = if self.running_tps_average == Duration::default() {
            delta
        } else {
            Duration::from_millis(
                (CLOCK_SMOOTHING * self.running_tps_average.as_millis() as f64
                    + (1.0 - CLOCK_SMOOTHING) * delta.as_millis() as f64) as u64,
            )
        };
    }
}
