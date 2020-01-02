use common::util::{linear_to_srgba, srgba_to_linear};
/// Pixel art scaling
/// Note: The current ui is locked to the pixel grid with little animation, if we want smoothly
/// moving pixel art this should be done in the shaders
use image::RgbaImage;
use vek::*;

const EPSILON: f32 = 0.0001;

// Averaging colors with alpha such that when blending with the background color the same color
// will be produced as when the individual colors were blended with the background and then
// averaged
// Say we have two areas that we are combining to form a single pixel
// A1 and A2 where these are the fraction of the area of the pixel each color contributes to
// Then if the colors were opaque we would say that the final color ouput color o3 is
//     E1: o3 = A1 * o1 + A2 * o2
// where o1 and o2 are the opaque colors of the two areas
// now say the areas are actually translucent and these opaque colors are derived by blending with a
// common backgound color b
//     E2: o1 = c1 * a1 + g * (1 - a1)
//     E3: o2 = c2 * a2 + g * (1 - a2)
// we want to find the combined color (c3) and combined alpha (a3) such that
//     E4: o3 = c3 * a3 + g * (1 - a3)
// substitution of E2 and E3 into E1 gives
//     E5: o3 = A1 * (c1 * a1 + g * (1 - a1)) + A2 * (c2 * a2 + g * (1 - a2))
// combining E4 and E5 then separting like terms into separte equations gives
//     E6: c3 * a3 = A1 * c1 * a1 + A2 * c2 * a2
//     E7: g * (1 - a3) = A1 * g * (1 - a1) + A2 * g * (1 - a2)
// dropping g from E7 and solving for a3
//     E8: a3 = 1 - A1 * (1 - a1) + A2 * (1 - a2)
// we can now calculate the combined alpha value
// and E6 can then be solved for c3
//     E9: c3 = (A1 * c1 * a1 + A2 * c2 * a2) / a3
pub fn resize_pixel_art(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    let (width, height) = image.dimensions();
    let mut new_image = RgbaImage::new(new_width, new_height);

    // Ratio of old image dimensions to new dimensions
    // Also the sampling dimensions within the old image for a single pixel in the new image
    let wratio = width as f32 / new_width as f32;
    let hratio = height as f32 / new_height as f32;

    for x in 0..new_width {
        // Calculate sampling strategy
        let xsmin = x as f32 * wratio;
        let xsmax = xsmin + wratio;
        // Min and max pixels covered
        let xminp = xsmin.floor() as u32;
        let xmaxp = ((xsmax - EPSILON).ceil() as u32)
            .checked_sub(1)
            .unwrap_or(0);
        // Fraction of first pixel to use
        let first_x_frac = if xminp != xmaxp {
            1.0 - xsmin.fract()
        } else {
            xsmax - xsmin
        };
        let last_x_frac = xsmax - xmaxp as f32;
        for y in 0..new_height {
            // Calculate sampling strategy
            let ysmin = y as f32 * hratio;
            let ysmax = ysmin + hratio;
            // Min and max of pixels covered
            let yminp = ysmin.floor() as u32;
            let ymaxp = ((ysmax - EPSILON).ceil() as u32)
                .checked_sub(1)
                .unwrap_or(0);
            // Fraction of first pixel to use
            let first_y_frac = if yminp != ymaxp {
                1.0 - ysmin.fract()
            } else {
                ysmax - ysmin
            };
            let last_y_frac = ysmax - ymaxp as f32;

            let mut linear_color = Rgba::new(0.0, 0.0, 0.0, wratio * hratio);
            // Left column
            // First pixel sample (top left assuming that is the origin)
            linear_color += get_linear_with_frac(image, xminp, yminp, first_x_frac * first_y_frac);
            // Left edge
            for j in yminp + 1..ymaxp {
                linear_color += get_linear_with_frac(image, xminp, j, first_x_frac);
            }
            // Bottom left corner
            if yminp != ymaxp {
                linear_color +=
                    get_linear_with_frac(image, xminp, ymaxp, first_x_frac * last_y_frac);
            }
            // Interior columns
            for i in xminp + 1..xmaxp {
                // Top edge
                linear_color += get_linear_with_frac(image, i, yminp, first_y_frac);
                // Inner (entire pixel is encompassed by sample)
                for j in yminp + 1..ymaxp {
                    linear_color += get_linear_with_frac(image, i, j, 1.0);
                }
                // Bottom edge
                if yminp != ymaxp {
                    linear_color += get_linear_with_frac(image, xminp, ymaxp, last_y_frac);
                }
            }
            // Right column
            if xminp != xmaxp {
                // Top right corner
                linear_color +=
                    get_linear_with_frac(image, xmaxp, yminp, first_y_frac * last_x_frac);
                // Right edge
                for j in yminp + 1..ymaxp {
                    linear_color += get_linear_with_frac(image, xmaxp, j, last_x_frac);
                }
                // Bottom right corner
                if yminp != ymaxp {
                    linear_color +=
                        get_linear_with_frac(image, xmaxp, ymaxp, last_x_frac * last_y_frac);
                }
            }
            // Divide summed color by area sample covers and convert back to srgb
            // I wonder if precalulating the inverse of these divs would have a significant effect
            linear_color = linear_color / wratio / hratio;
            linear_color =
                Rgba::from_translucent(linear_color.rgb() / linear_color.a, linear_color.a);
            new_image.put_pixel(
                x,
                y,
                image::Rgba(
                    linear_to_srgba(linear_color)
                        .map(|e| (e * 255.0).round() as u8)
                        .into_array(),
                ),
            );
        }
    }
    new_image
}

fn get_linear(image: &RgbaImage, x: u32, y: u32) -> Rgba<f32> {
    srgba_to_linear(Rgba::<u8>::from(image.get_pixel(x, y).0).map(|e| e as f32 / 255.0))
}

// See comments above resize_pixel_art
fn get_linear_with_frac(image: &RgbaImage, x: u32, y: u32, frac: f32) -> Rgba<f32> {
    let rgba = get_linear(image, x, y);
    let adjusted_rgb = rgba.rgb() * rgba.a * frac;
    let adjusted_alpha = -frac * (1.0 - rgba.a);
    Rgba::from_translucent(adjusted_rgb, adjusted_alpha)
}
