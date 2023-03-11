use common_base::span;
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
    /// Last time `tick` was called
    last_sys_time: Instant,
    /// Will be calculated in `tick` returns the dt used by the next iteration
    /// of the main loop
    last_dt: Duration,
    /// Summed up `last_dt`
    total_tick_time: Duration,
    // Stats only
    // uses f32 so we have enough precision to display fps values while saving space
    // This is in seconds
    last_dts: VecDeque<NotNan<f32>>,
    last_dts_sorted: Vec<NotNan<f32>>,
    last_busy_dts: VecDeque<NotNan<f32>>,
    stats: ClockStats,
}

pub struct ClockStats {
    /// Busy dt is the part of the tick that we didn't sleep.
    /// e.g. the total tick is 33ms, including 25ms sleeping. then this returns
    /// 8ms
    /// This is in seconds
    pub average_busy_dt: Duration,
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
const NUMBER_OF_DELTAS_COMPARED: usize = 5;

impl Clock {
    pub fn new(target_dt: Duration) -> Self {
        Self {
            target_dt,
            last_sys_time: Instant::now(),
            last_dt: target_dt,
            total_tick_time: Duration::default(),
            last_dts: VecDeque::with_capacity(NUMBER_OF_OLD_DELTAS_KEPT),
            last_dts_sorted: Vec::with_capacity(NUMBER_OF_OLD_DELTAS_KEPT),
            last_busy_dts: VecDeque::with_capacity(NUMBER_OF_OLD_DELTAS_KEPT),
            stats: ClockStats::new(&[], &VecDeque::new()),
        }
    }

    pub fn set_target_dt(&mut self, target_dt: Duration) { self.target_dt = target_dt; }

    pub fn stats(&self) -> &ClockStats { &self.stats }

    pub fn dt(&self) -> Duration { self.last_dt }

    pub fn get_stable_dt(&self) -> Duration {
        if self.last_dts.len() >= NUMBER_OF_DELTAS_COMPARED {
            // Take the median of the last few tick times
            let mut dts = [0.0; NUMBER_OF_DELTAS_COMPARED];
            for (i, dt) in self
                .last_dts
                .iter()
                .rev()
                .take(NUMBER_OF_DELTAS_COMPARED)
                .enumerate()
            {
                dts[i] = **dt;
            }
            dts.sort_by_key(|x| ordered_float::OrderedFloat(*x));
            let stable_dt = Duration::from_secs_f32(dts[NUMBER_OF_DELTAS_COMPARED / 2]);

            if self.last_dt > 2 * stable_dt {
                tracing::trace!(?self.last_dt, ?self.total_tick_time, "lag spike detected, unusually slow tick");
                stable_dt
            } else {
                self.last_dt
            }
        } else {
            self.last_dt
        }
    }

    /// Do not modify without asking @xMAC94x first!
    pub fn tick(&mut self) {
        span!(_guard, "tick", "Clock::tick");
        span!(guard, "clock work");
        let current_sys_time = Instant::now();
        let busy_delta = current_sys_time.duration_since(self.last_sys_time);
        // Maintain TPS
        self.last_dts_sorted = self.last_dts.iter().copied().collect();
        self.last_dts_sorted.sort_unstable();
        self.stats = ClockStats::new(&self.last_dts_sorted, &self.last_busy_dts);
        drop(guard);
        // Attempt to sleep to fill the gap.
        if let Some(sleep_dur) = self.target_dt.checked_sub(busy_delta) {
            spin_sleep::sleep(sleep_dur);
        }

        let after_sleep_sys_time = Instant::now();
        self.last_dt = after_sleep_sys_time.duration_since(self.last_sys_time);
        if self.last_dts.len() >= NUMBER_OF_OLD_DELTAS_KEPT {
            self.last_dts.pop_front();
        }
        if self.last_busy_dts.len() >= NUMBER_OF_OLD_DELTAS_KEPT {
            self.last_busy_dts.pop_front();
        }
        self.last_dts.push_back(
            NotNan::new(self.last_dt.as_secs_f32())
                .expect("Duration::as_secs_f32 never returns NaN"),
        );
        self.last_busy_dts.push_back(
            NotNan::new(busy_delta.as_secs_f32()).expect("Duration::as_secs_f32 never returns NaN"),
        );
        self.total_tick_time += self.last_dt;
        self.last_sys_time = after_sleep_sys_time;
    }
}

impl ClockStats {
    fn new(sorted: &[NotNan<f32>], busy_dt_list: &VecDeque<NotNan<f32>>) -> Self {
        let average_frame_time =
            sorted.iter().sum::<NotNan<f32>>().into_inner() / sorted.len().max(1) as f32;

        let average_busy_dt = busy_dt_list.iter().sum::<NotNan<f32>>().into_inner()
            / busy_dt_list.len().max(1) as f32;

        let average_tps = 1.0 / average_frame_time as f64;
        let (median_tps, percentile_90_tps, percentile_95_tps, percentile_99_tps) =
            if sorted.len() >= NUMBER_OF_OLD_DELTAS_KEPT {
                let median_frame_time = *sorted[sorted.len() / 2];
                let percentile_90_frame_time =
                    *sorted[(NUMBER_OF_OLD_DELTAS_KEPT as f32 * 0.1) as usize];
                let percentile_95_frame_time =
                    *sorted[(NUMBER_OF_OLD_DELTAS_KEPT as f32 * 0.05) as usize];
                let percentile_99_frame_time =
                    *sorted[(NUMBER_OF_OLD_DELTAS_KEPT as f32 * 0.01) as usize];

                let median_tps = 1.0 / median_frame_time as f64;
                let percentile_90_tps = 1.0 / percentile_90_frame_time as f64;
                let percentile_95_tps = 1.0 / percentile_95_frame_time as f64;
                let percentile_99_tps = 1.0 / percentile_99_frame_time as f64;
                (
                    median_tps,
                    percentile_90_tps,
                    percentile_95_tps,
                    percentile_99_tps,
                )
            } else {
                let avg_tps = 1.0 / average_busy_dt as f64;
                (avg_tps, avg_tps, avg_tps, avg_tps)
            };

        Self {
            average_busy_dt: Duration::from_secs_f32(average_busy_dt),
            average_tps,
            median_tps,
            percentile_90_tps,
            percentile_95_tps,
            percentile_99_tps,
        }
    }
}
