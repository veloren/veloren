use common_base::span;
use std::time::{Duration, Instant};
use vek::Lerp;

/// A type for maintaining consistent tick/frame pacing.
pub struct Clock {
    // Inputs
    /// The time the clock was started
    start: Instant,
    /// This is the dt that the Clock tries to archive with each call of tick.
    target_dt: Duration,

    // Working state
    /// The last time the clock was ticked
    last_tick: Instant,
    /// The last time we started performing work
    last_work: Instant,
    /// The number of ticks that have elapsed so far
    tick: u64,
    /// The interval that the last tick occurred on
    last_interval: u64,

    // Outputs
    /// The average time between ticks, seconds
    average_dt: f64,
    /// The average amount of time within each tick in which we're busy (i.e:
    /// not sleeping)
    average_busy: f64,
    /// The time that passed between the last tick, and the tick before it
    last_dt: f64,
}

pub struct ClockStats {
    /// A weighted average of the recent 'busy period' (i.e: time spent doing
    /// work rather than sleeping) per tick.
    pub average_busy_dt: Duration,
    /// A weighted average of the recent number of ticks per second.
    pub average_tps: f64,
}

// The weighting used to calculate averages. Must be > 0.0. 1.0 = no averaging.
const SMOOTH_WEIGHT: f64 = 0.1;
// The number of ticks over which catchup might occur after a stutter.
const CATCHUP_TICKS: u64 = 4;

impl Clock {
    pub fn new(target_dt: Duration) -> Self {
        Self {
            start: Instant::now(),
            target_dt,

            last_tick: Instant::now(),
            last_work: Instant::now(),
            tick: 0,
            last_interval: 0,

            average_dt: target_dt.as_secs_f64(),
            average_busy: target_dt.as_secs_f64(),
            last_dt: target_dt.as_secs_f64(),
        }
    }

    pub fn set_target_dt(&mut self, target_dt: Duration) { self.target_dt = target_dt; }

    pub fn stats(&self) -> ClockStats {
        ClockStats {
            average_busy_dt: Duration::from_secs_f64(self.average_busy),
            average_tps: 1.0 / self.average_dt.max(0.000001),
        }
    }

    pub fn dt(&self) -> Duration { Duration::from_secs_f64(self.last_dt) }

    pub fn get_stable_dt(&self) -> Duration { Duration::from_secs_f64(self.average_dt) }

    pub fn tick(&mut self) {
        span!(_guard, "tick", "Clock::tick");
        span!(guard, "clock work");

        if self.tick % 60 == 0 {
            // Give the tick thread realtime priority to minimise stuttering. Don't do this
            // all the time to avoid upsetting the scheduler.
            use thread_priority::*;
            _ = std::thread::current().set_priority_and_policy(
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline),
                // We choose scheduler parameters based on averages from previous frames
                ThreadPriority::Deadline {
                    runtime: Duration::from_secs_f64(
                        self.average_busy.min(self.target_dt.as_secs_f64()),
                    ),
                    deadline: Duration::from_secs_f64(self.target_dt.as_secs_f64() * 0.25),
                    period: self.target_dt,
                    flags: Default::default(),
                },
            );
        }

        let this_tick = Instant::now();

        // Calculate average metrics
        let busy_time = self.last_work.elapsed().as_secs_f64();
        self.average_busy = Lerp::lerp(self.average_busy, busy_time, SMOOTH_WEIGHT);
        let tick_time = self.last_tick.elapsed().as_secs_f64();
        self.average_dt = Lerp::lerp(self.average_dt, tick_time, SMOOTH_WEIGHT);

        drop(guard);

        // Rather than calculating when the next frame should be with 'dead reckoning',
        // we instead try to pace ticks at precise intervals. First, we work out
        // which interval we're in according to real time.
        let real_interval =
            (self.start.elapsed().as_secs_f64() / self.target_dt.as_secs_f64()) as u64;
        // Next, we determine the interval that we're going to pace the next tick for.
        // Ideally this would just be the last interval + 1, but in practice we
        // may need to skip intervals to catch up if we're lagging.
        let next_interval = (self.last_interval + 1)
            // If we're too slow, we might need to skip intervals. But this can result in
            // micro-stutters (skipped frames). Since rendering is double or even triple-buffered,
            // we can use a few ticks to try to catch up with real time
            .max(real_interval.saturating_sub(CATCHUP_TICKS))
            // Ensure we don't pace ourselves ahead of real time
            .min(real_interval + 1);
        // Now, calculate when the next interval will be in time
        let next_interval_time = self.start
            + Duration::from_secs_f64(next_interval as f64 * self.target_dt.as_secs_f64());
        // If there's still time to go until the next interval, sleep until it arrives.
        // Otherwise, full steam ahead into the next tick - we're already behind
        // schedule!
        if let Some(sleep_dur) = next_interval_time.checked_duration_since(this_tick) {
            spin_sleep::sleep(sleep_dur);
        }

        self.last_tick = this_tick;
        self.last_work = Instant::now();
        self.last_dt = tick_time;
        self.last_interval = next_interval;
        self.tick += 1;
    }
}
