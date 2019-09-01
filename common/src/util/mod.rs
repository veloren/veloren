pub const GIT_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/githash"));

use vek::{Rgb, Rgba, Vec3};

#[inline(always)]
pub fn srgb_to_linear(col: Rgb<f32>) -> Rgb<f32> {
    #[inline(always)]
    fn to_linear(x: f32) -> f32 {
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    }
    col.map(to_linear)
}
#[inline(always)]
pub fn linear_to_srgb(col: Rgb<f32>) -> Rgb<f32> {
    #[inline(always)]
    fn to_srgb(x: f32) -> f32 {
        if x <= 0.0031308 {
            x * 12.92
        } else {
            x.powf(1.0 / 2.4) * 1.055 - 0.055
        }
    }
    col.map(to_srgb)
}
#[inline(always)]
pub fn srgba_to_linear(col: Rgba<f32>) -> Rgba<f32> {
    Rgba::from_translucent(srgb_to_linear(Rgb::from(col)), col.a)
}
#[inline(always)]
pub fn linear_to_srgba(col: Rgba<f32>) -> Rgba<f32> {
    Rgba::from_translucent(linear_to_srgb(Rgb::from(col)), col.a)
}

/// Convert rgb to hsv. Expects rgb to be [0, 1].
#[inline(always)]
pub fn rgb_to_hsv(rgb: Rgb<f32>) -> Vec3<f32> {
    let (r, g, b) = rgb.into_tuple();
    let (max, min, diff, add) = {
        let (max, min, diff, add) = if r > g {
            (r, g, g - b, 0.0)
        } else {
            (g, r, b - r, 2.0)
        };
        if b > max {
            (b, min, r - g, 4.0)
        } else {
            (max, b.min(min), diff, add)
        }
    };

    let v = max;
    let h = if max == min {
        0.0
    } else {
        let mut h = 60.0 * (add + diff / (max - min));
        if h < 0.0 {
            h += 360.0;
        }
        h
    };
    let s = if max == 0.0 { 0.0 } else { (max - min) / max };

    Vec3::new(h, s, v)
}
/// Convert hsv to rgb. Expects h [0, 360], s [0, 1], v [0, 1]
#[inline(always)]
pub fn hsv_to_rgb(hsv: Vec3<f32>) -> Rgb<f32> {
    let (h, s, v) = hsv.into_tuple();
    let c = s * v;
    let h = h / 60.0;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h >= 0.0 && h <= 1.0 {
        (c, x, 0.0)
    } else if h <= 2.0 {
        (x, c, 0.0)
    } else if h <= 3.0 {
        (0.0, c, x)
    } else if h <= 4.0 {
        (0.0, x, c)
    } else if h <= 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Rgb::new(r + m, g + m, b + m)
}

#[inline(always)]
pub fn saturate_srgb(col: Rgb<f32>, value: f32) -> Rgb<f32> {
    let mut hsv = rgb_to_hsv(srgb_to_linear(col));
    hsv.y *= 1.0 + value;
    linear_to_srgb(hsv_to_rgb(hsv).map(|e| e.min(1.0).max(0.0)))
}
