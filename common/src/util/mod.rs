pub const GIT_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/githash"));

use palette::{Hsv, Saturate, Srgb};
use vek::{Rgb, Rgba};

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
#[inline(always)]
pub fn saturate_srgb(col: Rgb<f32>, value: f32) -> Rgb<f32> {
    Rgb::from(
        Srgb::from(Hsv::from(Srgb::from_components(col.into_tuple())).saturate(value))
            .into_components(),
    )
    .map(|c: f32| (c.max(0.0).min(1.0)))
    .into()
}
