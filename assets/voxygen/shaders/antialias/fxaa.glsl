#include <fxaa.glsl>

vec4 aa_apply(
    texture2D tex, sampler smplr,
    texture2D depth_tex, sampler depth_smplr,
    vec2 fragCoord,
    vec2 resolution
) {
    return fxaa_apply(tex, smplr, fragCoord, resolution, 1.0);
}
