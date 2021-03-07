use std::ops::Range;
use vek::*;

/// Return a value between 0 and 1 corresponding to how close to the centre of
/// `range` `x` is. The exact function used is left unspecified, but it shall
/// have the shape of a bell-like curve. This function is required to return `0`
/// (or a value extremely close to `0`) when `x` is outside of `range`.
pub fn close(x: f32, range: Range<f32>) -> f32 {
    let mean = (range.start + range.end) / 2.0;
    let width = (range.end - range.start) / 2.0;
    (1.0 - ((x - mean) / width).clamped(-1.0, 1.0).powi(2)).powi(2)
}
