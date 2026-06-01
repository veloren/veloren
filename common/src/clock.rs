use common_base::span;
use std::time::{Duration, Instant};
use vek::Lerp;

/// A type for maintaining consistent tick/frame pacing.
pub struct Clock {
    // Inputs
    /// This is the dt that the Clock tries to archive with each call of tick.
    target_dt: Duration,

    // Working state
    /// The last time the clock was ticked
    last_tick: Instant,
    /// The last time we started performing work
    last_work: Instant,
    /// The number of ticks that have elapsed so far
    tick: u64,

    // Outputs
    /// The average time between ticks, seconds
    average_dt: f64,
    /// The average amount of time within each tick in which we're busy (i.e:
    /// not sleeping)
    average_busy: f64,
    /// The average amount of variance between ticks
    average_variance: f64,
    /// The time that passed between the last tick, and the tick before it
    last_dt: f64,
}

pub struct ClockStats {
    /// A weighted average of the recent 'busy period' (i.e: time spent doing
    /// work rather than sleeping) per tick.
    pub average_busy_dt: Duration,
    /// A weighted average of the recent number of ticks per second.
    pub average_tps: f64,
    /// A weighted average of the variance of the clock relative to the average
    /// TPS.
    pub average_variance: Duration,
}

/// The weighting used to calculate averages. Must be > 0.0. 1.0 = no averaging.
const SMOOTH_WEIGHT: f64 = 0.025;

impl Clock {
    pub fn new(target_dt: Duration) -> Self {
        Self {
            target_dt,

            last_tick: Instant::now(),
            last_work: Instant::now(),
            tick: 0,

            average_dt: target_dt.as_secs_f64(),
            average_busy: target_dt.as_secs_f64(),
            average_variance: 0.0,
            last_dt: target_dt.as_secs_f64(),
        }
    }

    pub fn set_target_dt(&mut self, target_dt: Duration) {
        if target_dt != self.target_dt {
            self.target_dt = target_dt;
            // The target dt has changed, throw out the existing state to avoid problems
            self.average_dt = target_dt.as_secs_f64();
            self.average_busy = target_dt.as_secs_f64();
            self.average_variance = 0.0;
        }
    }

    pub fn stats(&self) -> ClockStats {
        ClockStats {
            average_busy_dt: Duration::from_secs_f64(self.average_busy),
            average_tps: 1.0 / self.average_dt.max(0.000001),
            average_variance: Duration::from_secs_f64(self.average_variance),
        }
    }

    pub fn dt(&self) -> Duration { Duration::from_secs_f64(self.last_dt) }

    pub fn get_stable_dt(&self) -> Duration { Duration::from_secs_f64(self.average_dt) }

    pub fn tick(&mut self) {
        span!(_guard, "tick", "Clock::tick");
        span!(guard, "clock work");

        // Give the tick thread realtime priority to minimise stuttering. Don't do this
        // all the time to avoid upsetting the scheduler.
        if self.tick % 30 == 0 {
            use thread_priority::*;
            // Try to target a tick period that's consistent with our current FPS (a low but
            // consistent framerate is a better outcome than one that's faster on paper but
            // is bouncing around all over the place).
            let stable_dt = self.average_busy
                // Don't try to schedule for a tick rate that's higher than our target, even if we
                // could achieve it.
                .max(self.target_dt.as_secs_f64());
            _ = std::thread::current().set_priority_and_policy(
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo),
                // ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline),
                ThreadPriority::Crossplatform(ThreadPriorityValue::try_from(90).unwrap()),
                // // We choose scheduler parameters based on averages from previous frames
                // ThreadPriority::Deadline {
                //     runtime: Duration::from_secs_f64(self.average_busy * 0.5),
                //     deadline: Duration::from_millis(10),
                //     period: Duration::from_secs_f64(stable_dt),
                //     flags: Default::default(),
                // },
            );
        }

        let this_tick = Instant::now();

        // Calculate average metrics
        let busy_time = self.last_work.elapsed().as_secs_f64();
        self.average_busy = Lerp::lerp(self.average_busy, busy_time, SMOOTH_WEIGHT);
        let tick_time = self.last_tick.elapsed().as_secs_f64();
        self.average_dt = Lerp::lerp(self.average_dt, tick_time, SMOOTH_WEIGHT);
        let variance = (tick_time - self.average_dt).abs();
        self.average_variance = Lerp::lerp(self.average_variance, variance, SMOOTH_WEIGHT);

        drop(guard);

        // Sleep for any remaining time before the next tick
        if let Some(sleep_dur) = self
            .target_dt
            .checked_sub(Duration::from_secs_f64(busy_time))
        {
            spin_sleep::sleep(sleep_dur);
        }

        self.last_tick = this_tick;
        self.last_work = Instant::now();
        self.last_dt = tick_time;
        self.tick += 1;
    }
}
