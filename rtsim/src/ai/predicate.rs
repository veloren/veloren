use rand::RngExt;

use super::NpcCtx;

pub trait Predicate: Sized + Clone {
    fn should(&mut self, ctx: &mut NpcCtx) -> bool;

    fn chance(self, chance: f32) -> Chance<Self> {
        Chance {
            predicate: self,
            chance,
        }
    }

    /// Hint for when this will be true.
    fn time_hint(&self) -> Option<f32> { None }
}

#[derive(Clone, Copy)]
pub struct Yes;

impl Predicate for Yes {
    fn should(&mut self, _ctx: &mut NpcCtx) -> bool { true }

    fn time_hint(&self) -> Option<f32> { Some(0.0) }
}

#[derive(Clone)]
pub struct EveryRange {
    next: Option<f32>,
    sample: std::ops::Range<f32>,
}

pub fn every_range(r: std::ops::Range<f32>) -> EveryRange {
    EveryRange {
        next: None,
        sample: r,
    }
}

impl Predicate for EveryRange {
    fn should(&mut self, ctx: &mut NpcCtx) -> bool {
        if let Some(ref mut next) = self.next {
            *next -= ctx.dt;
            if *next <= 0.0 {
                *next += ctx.rng.random_range(self.sample.clone());
                true
            } else {
                false
            }
        } else {
            self.next = Some(ctx.rng.random_range(self.sample.clone()));
            false
        }
    }

    fn time_hint(&self) -> Option<f32> { self.next }
}

#[derive(Clone, Copy)]
pub struct Chance<P> {
    predicate: P,
    chance: f32,
}

impl<P: Predicate> Predicate for Chance<P> {
    fn should(&mut self, ctx: &mut NpcCtx) -> bool {
        self.predicate.should(ctx) && ctx.rng.random_bool(self.chance as f64)
    }

    fn time_hint(&self) -> Option<f32> { self.predicate.time_hint() }
}

impl<F: Fn(&mut NpcCtx) -> bool + Clone> Predicate for F {
    fn should(&mut self, ctx: &mut NpcCtx) -> bool { self(ctx) }
}

// Seconds
pub fn timeout(time: f64) -> Timeout { Timeout { seconds_left: time } }

#[derive(Clone, Copy)]
pub struct Timeout {
    seconds_left: f64,
}

impl Predicate for Timeout {
    fn should(&mut self, ctx: &mut NpcCtx) -> bool {
        self.seconds_left -= ctx.dt as f64;
        self.seconds_left <= 0.0
    }

    fn time_hint(&self) -> Option<f32> { Some(self.seconds_left.max(0.0) as f32) }
}
