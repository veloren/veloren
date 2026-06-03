use common_base::span;
use std::time::{Duration, Instant};
use vek::Lerp;

/// A type for maintaining consistent tick/frame pacing.
pub struct Clock {
    // Inputs
    /// This is the dt that the Clock tries to archive with each call of tick.
    target_dt: Duration,

    // Working state
    /// The amount of real time that has passed on the clock
    real_time: Duration,
    /// The amount of game time that has passed on the clock
    game_time: Duration,
    /// The last time the clock was ticked
    last_tick: Instant,
    /// The last time we started performing work
    last_work: Instant,
    /// The number of ticks that have elapsed so far
    tick: u64,

    /// The average time between ticks, seconds
    average_dt: f64,
    /// The average amount of time within each tick in which we're busy (i.e:
    /// not sleeping)
    average_busy: f64,
    /// The average amount of variance between ticks
    average_variance: f64,
    /// The time that passed between the last tick, and the tick before it
    last_real_dt: f64,
    /// The dt to be used for the next game tick, in game time.
    last_game_dt: f64,
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
const SMOOTH_WEIGHT: f64 = 0.05;
/// The proportion of the difference between real and game time that gets
/// applied each tick to keep the two aligned.
const NUDGE_RATE: f64 = 0.05;
/// The maximum dt that the game should ever run at.
const MAX_GAME_DT: f64 = 1.0 / 5.0;

impl Clock {
    pub fn new(target_dt: Duration) -> Self {
        Self {
            target_dt,

            real_time: Duration::ZERO,
            game_time: Duration::ZERO,
            last_tick: Instant::now(),
            last_work: Instant::now(),
            tick: 0,

            average_dt: target_dt.as_secs_f64(),
            average_busy: target_dt.as_secs_f64(),
            average_variance: 0.0,
            last_real_dt: target_dt.as_secs_f64(),
            last_game_dt: target_dt.as_secs_f64(),
        }
    }

    pub fn set_target_dt(&mut self, target_dt: Duration) {
        if target_dt != self.target_dt {
            self.target_dt = target_dt;

            // The target dt has changed, throw out the existing stats to avoid problems
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

    pub fn real_dt(&self) -> Duration { Duration::from_secs_f64(self.last_real_dt) }

    pub fn game_dt(&self) -> Duration { Duration::from_secs_f64(self.last_game_dt) }

    pub fn tick(&mut self) {
        span!(_guard, "tick", "Clock::tick");
        span!(guard, "clock work");

        // Give the tick thread realtime priority to minimise stuttering. Don't do this
        // all the time to avoid upsetting the scheduler.
        if self.tick == 0
        /* .is_multiple_of(30) */
        {
            use thread_priority::*;
            // // We choose scheduler parameters based on averages from previous frames
            // // Try to target a tick period that's consistent with our current FPS (a low
            // but // consistent framerate is a better outcome than one that's
            // faster on paper but // is bouncing around all over the place).
            // let stable_dt = self.average_busy
            //     // Don't try to schedule for a tick rate that's higher than our target,
            // even if we     // could achieve it.
            //     .max(self.target_dt.as_secs_f64());
            // let priority = ThreadPriority::Deadline {
            //     runtime: Duration::from_secs_f64(self.average_busy * 0.5),
            //     deadline: Duration::from_millis(10),
            //     period: Duration::from_secs_f64(stable_dt),
            //     flags: Default::default(),
            // };
            let priority =
                ThreadPriority::Crossplatform(ThreadPriorityValue::try_from(90).unwrap());
            _ = cfg_select! {
                unix => std::thread::current().set_priority_and_policy(
                    // ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline),
                    ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo),
                    priority,
                ),
                _ => std::thread::current().set_priority(priority),
            };
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

        // Update clock state

        self.last_tick = this_tick;
        self.last_work = Instant::now();

        // Progress real and game time
        self.real_time += Duration::from_secs_f64(self.last_real_dt);
        self.game_time += Duration::from_secs_f64(self.last_game_dt);

        // Calculate the deltas for both real and game clocks. The real clock is
        // absolute: we can't alter the progression of time. However, we can
        // alter the game clock and nudge is toward real time. The reason we
        // don't want to keep the two *exactly* in time is that a lag spike on a
        // single tick would cause a corresponding jump in dt on the next tick, which
        // might produce strange results for any dt-dependent gameplay systems.
        // Instead, we gradually nudge the game time back toward real time over
        // several ticks.
        self.last_real_dt = tick_time;
        self.last_game_dt = (self.average_dt
            + (self.real_time.as_secs_f64() - self.game_time.as_secs_f64()) * NUDGE_RATE)
            .min(MAX_GAME_DT);

        self.tick += 1;
    }
}
