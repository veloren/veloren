use std::f32::consts::PI;

// Useful easing functions
// Source: https://easings.net/
pub fn bounce(x: f32) -> f32 {
    if x < (1.0 / 2.75) {
        7.5625 * x.powi(2)
    } else if x < (2.0 / 2.75) {
        7.5625 * (x - (1.5 / 2.75)).powi(2) + 0.75
    } else if x < (2.5 / 2.75) {
        7.5625 * (x - (2.25 / 2.75)).powi(2) + 0.9375
    } else {
        7.5625 * (x - (2.625 / 2.75)).powi(2) + 0.984375
    }
}

// Source: https://easings.net/
pub fn elastic(x: f32) -> f32 {
    fn f(x: f32, a: f32, b: f32) -> f32 {
        let p = 0.8;
        b + a * 2.0_f32.powf(a * 10.0 * x) * ((4.0 * PI * x) / p).cos()
    }
    f(x, -1.0, 1.0) / f(1.0, -1.0, 1.0)
}

// Source: https://easings.net/
pub fn ease_in_back(x: f32) -> f32 {
    let a = 1.70158;
    let b = a + 1.0;
    b * x.powi(3) - a * x.powi(2)
}

pub fn out_and_in(x: f32) -> f32 { (x - 0.5).powi(2) - 0.25 }
