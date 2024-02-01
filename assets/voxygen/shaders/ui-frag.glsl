#version 440 core

#include <globals.glsl>
#include <constants.glsl>

layout(location = 0) in vec2 f_uv;
layout(location = 1) in vec4 f_color;
layout(location = 2) flat in vec2 f_scale;
layout(location = 3) flat in uint f_mode;

layout (std140, set = 1, binding = 0)
uniform u_locals {
    vec4 w_pos;
};

layout(set = 2, binding = 0)
uniform texture2D t_tex;
layout(set = 2, binding = 1)
uniform sampler s_tex;
layout (std140, set = 2, binding = 2)
uniform tex_locals {
    uvec2 texture_size;
};

layout(location = 0) out vec4 tgt_color;

// Adjusts the provided uv value to account for coverage of pixels from the
// sampled texture by the current fragment when upscaling.
//
// * `pos` - Position in the sampled texture in pixel coordinates. This is
//   where the center of the current fragment lies on the sampled texture.
// * `scale` - Scaling of pixels from the sampled texture to the render target.
//   This is the amount of fragments that each pixel from the sampled texture
//   covers.
float upscale_adjust(float pos, float scale) {
    // To retain crisp borders of upscaled pixel art, images are upscaled
    // following the algorithm outlined here:
    //
    // https://csantosbh.wordpress.com/2014/01/25/manual-texture-filtering-for-pixelated-games-in-webgl/
    // 
    // `min(x * scale, 0.5) + max((x - 1.0) * scale, 0.0)`
    //
    float frac = fract(pos);
    // Right of nearest pixel in the sampled texture.
    float base = floor(pos);
    // This will be 0.5 when the current fragment lies entirely inside a pixel
    // in the sampled texture.
    float adjustment = min(frac * scale, 0.5) + max((frac - 1.0) * scale + 0.5, 0.0);
    return base + adjustment;
}

// Computes info needed for downscaling using two samples in a single
// dimension. This info includes the two position to sample at (called
// `offsets` even though they aren't actually offsets from the supplied
// position) and the `weights` to multiply each of those samples by before
// combining them.
//
// See `upscale_adjust` for semantics of `pos` and `scale` parameters.
// 
// Ouput via `weights` and `offsets` parameters.
void downscale_params(float pos, float scale, out vec2 weights, out vec2 offsets) {
    // For `scale` 0.33333..1.0 we round to the nearest pixel edge and split
    // there. We compute the length of each side. Then the sampling point is
    // computed as this distance from the split point via this formula where
    // `l` is the length of that side of split:
    //
    // `1.5 - (1.0 / max(l, 1.0))`
    //
    // For `scale` ..0.3333 the current fragment can potentially span more than
    // 4 pixels (within a single dimension) in the sampled texture. So we can't
    // perfectly compute the contribution of each covered pixel in the sampled
    // texture with only 2 samples (along each dimension). Thus, we fallback to
    // an imperfect technique of just sampling 1 pixel length from the center
    // on each side of the nearest pixel edge. An alternative might be to
    // pre-compute mipmap levels that could be sampled from, although this
    // could interact poorly with the atlas.
    if (scale > (1.0 / 3.0)) {
        // Width of the fragment in terms of pixels in the sampled texture.
        float width = 1.0 / scale;
        // Right side of the fragment in the sampled texture.
        float right = pos - width / 2.0;
        float split = round(pos);
        float right_len = split - right;
        float left_len = width - right_len;
        float right_sample_offset = 1.5 - (1.0 / max(right_len, 1.0));
        float left_sample_offset = 1.5 - (1.0 / max(left_len, 1.0));
        offsets = vec2(split) + vec2(-right_sample_offset, left_sample_offset);
        weights = vec2(right_len, left_len) / width;
    } else {
        offsets = round(pos) + vec2(-1.0, 1.0);
        // We split in the middle so weights for both sides are the same.
        weights = vec2(0.5);
    }
}

// 1 sample
vec4 upscale_xy(vec2 uv_pixel, vec2 scale) {
    // When slowly panning something (e.g. the map), a very small amount of
    // wobbling is still observable (not as much as nearest sampling). It
    // is possible to eliminate this by making the edges slightly blurry by
    // lowering the scale a bit here. However, this does make edges little
    // less crisp and can cause bleeding in from other images packed into
    // the atlas in the current setup.
    vec2 adjusted = vec2(upscale_adjust(uv_pixel.x, scale.x), upscale_adjust(uv_pixel.y, scale.y));
    // Convert back to 0.0..1.0 by dividing by texture size.
    vec2 uv = adjusted / texture_size;
    return textureLod(sampler2D(t_tex, s_tex), uv, 0);
}

// 2 samples
vec4 upscale_x_downscale_y(vec2 uv_pixel, vec2 scale) {
    float x_adjusted = upscale_adjust(uv_pixel.x, scale.x);
    vec2 weights, offsets;
    downscale_params(uv_pixel.y, scale.y, weights, offsets);
    vec2 uv0 = vec2(x_adjusted, offsets[0]) / texture_size;
    vec2 uv1 = vec2(x_adjusted, offsets[1]) / texture_size;
    vec4 s0 = textureLod(sampler2D(t_tex, s_tex), uv0, 0);
    vec4 s1 = textureLod(sampler2D(t_tex, s_tex), uv1, 0);
    return s0 * weights[0] + s1 * weights[1];
}

// 2 samples
vec4 downscale_x_upscale_y(vec2 uv_pixel, vec2 scale) {
    float y_adjusted = upscale_adjust(uv_pixel.y, scale.y);
    vec2 weights, offsets;
    downscale_params(uv_pixel.x, scale.x, weights, offsets);
    vec2 uv0 = vec2(offsets[0], y_adjusted) / texture_size;
    vec2 uv1 = vec2(offsets[1], y_adjusted) / texture_size;
    vec4 s0 = textureLod(sampler2D(t_tex, s_tex), uv0, 0);
    vec4 s1 = textureLod(sampler2D(t_tex, s_tex), uv1, 0);
    return s0 * weights[0] + s1 * weights[1];
}

// 4 samples
vec4 downscale_xy(vec2 uv_pixel, vec2 scale) {
    vec2 weights_x, offsets_x, weights_y, offsets_y;
    downscale_params(uv_pixel.x, scale.x, weights_x, offsets_x);
    downscale_params(uv_pixel.y, scale.y, weights_y, offsets_y);
    vec2 uv0 = vec2(offsets_x[0], offsets_y[0]) / texture_size;
    vec2 uv1 = vec2(offsets_x[1], offsets_y[0]) / texture_size;
    vec2 uv2 = vec2(offsets_x[0], offsets_y[1]) / texture_size;
    vec2 uv3 = vec2(offsets_x[1], offsets_y[1]) / texture_size;
    vec4 s0 = textureLod(sampler2D(t_tex, s_tex), uv0, 0);
    vec4 s1 = textureLod(sampler2D(t_tex, s_tex), uv1, 0);
    vec4 s2 = textureLod(sampler2D(t_tex, s_tex), uv2, 0);
    vec4 s3 = textureLod(sampler2D(t_tex, s_tex), uv3, 0);
    vec4 s01 = s0 * weights_x[0] + s1 * weights_x[1];
    vec4 s23 = s2 * weights_x[0] + s3 * weights_x[1];
    // Useful to visualize things below the limit where downscaling is supposed
    // to be perfectly accurate.
    /*if (scale.x < (1.0 / 3.0)) {
        return vec4(1, 0, 0, 1);
    }*/
    return s01 * weights_y[0] + s23 * weights_y[1];
}

void main() {
    // Text
    if (f_mode == uint(0)) {
        // NOTE: This now uses linear filter since all `Texture::new_dynamic`
        // was changed to this by default. Glyphs are usually rasterized to be
        // pretty close to the target size (so the filter change may have no
        // effect), but there are thresholds within which the same rasterized
        // glyph will be re-used. I wasn't able to observe any differences.
        vec2 uv = f_uv;
        #ifdef EXPERIMENTAL_UINEARESTSCALING
            uv = (floor(uv * texture_size) + 0.5) / texture_size;
        #endif
        tgt_color = f_color * vec4(1.0, 1.0, 1.0, textureLod(sampler2D(t_tex, s_tex), uv, 0).a);
    // Image
    // HACK: bit 0 is set for both ordinary and north-facing images.
    } else if ((f_mode & uint(1)) == uint(1)) {
        // NOTE: We don't have to account for bleeding over the border of an image
        // due to how the ui currently handles rendering images. Currently, any
        // edges of an image being rendered that don't line up with a pixel are
        // snapped to a pixel, so we will never render any pixels containing an
        // image that lie partly outside that image (and thus the sampling here
        // will never try to sample outside an image). So we don't have to
        // worry about bleeding in the atlas and/or what the border behavior
        // should be.

        // Convert to sampled pixel coordinates.
        vec2 uv_pixel = f_uv * texture_size;
        vec4 image_color;
        #ifdef EXPERIMENTAL_UINEARESTSCALING
            vec2 uv = (floor(uv_pixel) + 0.5) / texture_size;
            image_color = textureLod(sampler2D(t_tex, s_tex), uv, 0);
        #else 
            if (f_scale.x >= 1.0) {
                if (f_scale.y >= 1.0) {
                    image_color = upscale_xy(uv_pixel, f_scale);     
                } else {
                    image_color = upscale_x_downscale_y(uv_pixel, f_scale);     
                }
            } else {
                if (f_scale.y >= 1.0) {
                    image_color = downscale_x_upscale_y(uv_pixel, f_scale);     
                } else {
                    image_color = downscale_xy(uv_pixel, f_scale);     
                }
            }
        #endif

        // un-premultiply alpha (linear filtering above requires alpha to be
        // pre-multiplied)
        if (image_color.a > 0.001) {
            image_color.rgb /= image_color.a;
        } 

        tgt_color = f_color * image_color;
    // 2D Geometry
    } else if (f_mode == uint(2)) {
        tgt_color = f_color;
    }
}
