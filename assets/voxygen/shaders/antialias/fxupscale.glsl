#include <fxaa.glsl>

vec4 aa_apply(
    texture2D tex, sampler smplr,
    texture2D depth_tex, sampler depth_smplr,
    vec2 fragCoord,
    vec2 resolution
) {
    vec4 aa_color = fxaa_apply(tex, smplr, fragCoord, resolution);

    vec2 sz = textureSize(sampler2D(tex, smplr), 0).xy;
    vec4 closest = vec4(1000);
    float closest_dist = 1000.0;
    ivec2 dirs[] = { ivec2(-1, 0), ivec2(1, 0), ivec2(0, -1), ivec2(0, 1), /*ivec2(-1, -1), ivec2(-1, 1), ivec2(1, -1), ivec2(1, 1)*/ };
    for (uint i = 0u; i < dirs.length(); i ++) {
        vec4 col_at = texelFetch(sampler2D(tex, smplr), ivec2(fragCoord / screen_res.xy * sz) + dirs[i], 0);
        float dist = dot(pow(aa_color.rgb - col_at.rgb, ivec3(2)), vec3(1));
        if (dist < closest_dist) {
            closest = col_at;
            closest_dist = dist;
        }
    }
    return mix(aa_color, closest, clamp(1.0 - sqrt(closest_dist), 0, 1));
}
