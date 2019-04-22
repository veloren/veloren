use vek::*;

#[inline(always)]
pub fn srgb_to_linear(c: Rgba<f32>) -> Rgba<f32> {
    #[inline(always)]
    fn to_linear(x: f32) -> f32 {
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    }
    Rgba {
        r: to_linear(c.r),
        g: to_linear(c.g),
        b: to_linear(c.b),
        a: c.a,
    }
}
#[inline(always)]
pub fn linear_to_srgb(c: Rgba<f32>) -> Rgba<f32> {
    #[inline(always)]
    fn to_srgb(x: f32) -> f32 {
        if x <= 0.0031308 {
            x * 12.92
        } else {
            x.powf(1.0 / 2.4) * 1.055 - 0.055
        }
    }
    Rgba {
        r: to_srgb(c.r),
        g: to_srgb(c.g),
        b: to_srgb(c.b),
        a: c.a,
    }
}
