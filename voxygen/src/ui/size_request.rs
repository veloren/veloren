// Standard
use std::ops::Add;

// Local
use super::Span;

pub struct SizeRequest {
    min: Span,
    max: Span,
}

impl SizeRequest {
    pub fn indifferent() -> Self {
        Self {
            min: Span::rel(0.0),
            max: Span::rel(std::f32::INFINITY),
        }
    }
}

impl Add<Span> for SizeRequest {
    type Output = Self;

    fn add(self, span: Span) -> Self {
        Self {
            min: self.min + span,
            max: self.max + span,
        }
    }
}
