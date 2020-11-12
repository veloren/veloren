use crate::span;
use ordered_float::NotNan;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// This Clock tries to make this tick a constant time by sleeping the rest of
/// the tick
/// - if we actually took less time than we planned: sleep and return planned
///   time
/// - if we ran behind: don't sleep and return actual time
/// We DON'T do any fancy averaging of the deltas for tick for 2 reasons:
///  - all Systems have to work based on `dt` and we cannot assume that this is
///    const through all ticks
///  - when we have a slow tick, a lag, it doesn't help that we have 10 fast
///    ticks directly afterwards
/// We return a smoothed version for display only!
pub struct Clock {
    /// This is the dt that the Clock tries to archive with each call of tick.
    target_dt: Duration,
    /// last time `tick` was called
    last_sys_time: Instant,
    /// will be calculated in `tick` returns the dt used by the next iteration
    /// of the main loop
    last_dt: Duration,
    /// summed up `last_dt`
    total_tick_time: Duration,
    // Stats only
    // uses f32 so we have enough precision to display fps values while saving space
    last_dts_millis: VecDeque<NotNan<f32>>,
    last_dts_millis_sorted: Vec<NotNan<f32>>,
    stats: ClockStats,
}

pub struct ClockStats {
    /// busy_dt is the part of the last tick that we didn't sleep.
    /// e.g. the total tick is 33ms, including 25ms sleeping. then this returns
    /// 8ms
    pub last_busy_dt: Duration,
    /// avg over the last NUMBER_OF_OLD_DELTAS_KEPT ticks
    pub average_tps: f64,
    /// = 50% percentile
    pub median_tps: f64,
    /// lowest 10% of the frames
    pub percentile_90_tps: f64,
    /// lowest 5% of the frames
    pub percentile_95_tps: f64,
    /// lowest 1% of the frames
    pub percentile_99_tps: f64,
}

const NUMBER_OF_OLD_DELTAS_KEPT: usize = 100;

impl Clock {
    pub fn new(target_dt: Duration) -> Self {
        Self {
            target_dt,
            last_sys_time: Instant::now(),
            last_dt: target_dt,
            total_tick_time: Duration::default(),
            last_dts_millis: VecDeque::with_capacity(NUMBER_OF_OLD_DELTAS_KEPT),
            last_dts_millis_sorted: Vec::with_capacity(NUMBER_OF_OLD_DELTAS_KEPT),
            stats: ClockStats::new(&Vec::new(), target_dt),
        }
    }

    pub fn set_target_dt(&mut self, target_dt: Duration) { self.target_dt = target_dt; }

    pub fn stats(&self) -> &ClockStats { &self.stats }

    pub fn dt(&self) -> Duration { self.last_dt }

    /// Do not modify without asking @xMAC94x first!
    pub fn tick(&mut self) {
        span!(_guard, "tick", "Clock::tick");
        span!(guard, "clock work");
        let current_sys_time = Instant::now();
        let busy_delta = current_sys_time.duration_since(self.last_sys_time);
        // Maintain TPS
        self.last_dts_millis_sorted = self.last_dts_millis.iter().copied().collect();
        self.last_dts_millis_sorted.sort_unstable();
        self.stats = ClockStats::new(&self.last_dts_millis_sorted, busy_delta);
        drop(guard);
        // Attempt to sleep to fill the gap.
        if let Some(sleep_dur) = self.target_dt.checked_sub(busy_delta) {
            spin_sleep::sleep(sleep_dur);
        }

        let after_sleep_sys_time = Instant::now();
        self.last_dt = after_sleep_sys_time.duration_since(self.last_sys_time);
        if self.last_dts_millis.len() >= NUMBER_OF_OLD_DELTAS_KEPT {
            self.last_dts_millis.pop_front();
        }
        self.last_dts_millis.push_back(
            NotNan::new(self.last_dt.as_secs_f32() * 1000.0)
                .expect("Duration::as_secs_f32 never returns NaN"),
        );
        self.total_tick_time += self.last_dt;
        self.last_sys_time = after_sleep_sys_time;
    }
}

impl ClockStats {
    fn new(sorted: &[NotNan<f32>], last_busy_dt: Duration) -> Self {
        const NANOS_PER_SEC: f64 = Duration::from_secs(1).as_nanos() as f64;
        const NANOS_PER_MILLI: f64 = Duration::from_millis(1).as_nanos() as f64;

        let len = sorted.len();
        let average_millis = sorted.iter().sum::<NotNan<f32>>().into_inner() / len.max(1) as f32;

        let average_tps = NANOS_PER_SEC / (average_millis as f64 * NANOS_PER_MILLI);
        let (median_tps, percentile_90_tps, percentile_95_tps, percentile_99_tps) = if len
            >= NUMBER_OF_OLD_DELTAS_KEPT
        {
            let median_millis = *sorted[len / 2];
            let percentile_90_millis = *sorted[(NUMBER_OF_OLD_DELTAS_KEPT as f32 * 0.1) as usize];
            let percentile_95_millis = *sorted[(NUMBER_OF_OLD_DELTAS_KEPT as f32 * 0.05) as usize];
            let percentile_99_millis = *sorted[(NUMBER_OF_OLD_DELTAS_KEPT as f32 * 0.01) as usize];

            let median_tps = NANOS_PER_SEC / (median_millis as f64 * NANOS_PER_MILLI);
            let percentile_90_tps = NANOS_PER_SEC / (percentile_90_millis as f64 * NANOS_PER_MILLI);
            let percentile_95_tps = NANOS_PER_SEC / (percentile_95_millis as f64 * NANOS_PER_MILLI);
            let percentile_99_tps = NANOS_PER_SEC / (percentile_99_millis as f64 * NANOS_PER_MILLI);
            (
                median_tps,
                percentile_90_tps,
                percentile_95_tps,
                percentile_99_tps,
            )
        } else {
            let avg_tps = NANOS_PER_SEC / last_busy_dt.as_nanos() as f64;
            (avg_tps, avg_tps, avg_tps, avg_tps)
        };

        Self {
            last_busy_dt,
            average_tps,
            median_tps,
            percentile_90_tps,
            percentile_95_tps,
            percentile_99_tps,
        }
    }
}
