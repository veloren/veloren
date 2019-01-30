// Standard
use std::ops::{Add, Sub};

#[derive(Copy, Clone)]
pub struct Span {
    pub rel: f32,
    pub abs: f32,
}

impl Span {
    pub fn rel(rel: f32) -> Self { Self { rel, abs: 0.0 } }
    pub fn abs(abs: f32) -> Self { Self { rel: 0.0, abs } }

    pub fn full() -> Self { Self { rel: 1.0, abs: 0.0 } }
    pub fn half() -> Self { Self { rel: 0.5, abs: 0.0 } }
    pub fn none() -> Self { Self { rel: 0.0, abs: 0.0 } }

    pub fn to_abs(self, res: f32) -> Self {
        Self { rel: 0.0, abs: self.rel * res + self.abs }
    }

    pub fn to_rel(self, res: f32) -> Self {
        Self { rel: self.rel + self.abs / res, abs: 0.0 }
    }
}

impl Add for Span {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            rel: self.rel + other.rel,
            abs: self.abs + other.abs,
        }
    }
}

impl Sub for Span {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            rel: self.rel - other.rel,
            abs: self.abs - other.abs,
        }
    }
}
