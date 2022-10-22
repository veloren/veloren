#version 420 core

#include <constants.glsl>

#define LIGHTING_TYPE (LIGHTING_TYPE_TRANSMISSION | LIGHTING_TYPE_REFLECTION)

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_SPECULAR

#if (FLUID_MODE == FLUID_MODE_CHEAP)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE >= FLUID_MODE_MEDIUM)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

// Must come before includes
#define IS_POSTPROCESS

#include <globals.glsl>
// Note: The sampler uniform is declared here because it differs for MSAA
#include <anti-aliasing.glsl>
#include <srgb.glsl>
#include <cloud.glsl>
#include <light.glsl>
// This *MUST* come after `cloud.glsl`: it contains a function that depends on `cloud.glsl` when clouds are enabled
#include <point_glow.glsl>
#include <random.glsl>

layout(set = 2, binding = 0)
uniform texture2D t_src_color;
layout(set = 2, binding = 1)
uniform sampler s_src_color;

layout(set = 2, binding = 2)
uniform texture2D t_src_depth;
layout(set = 2, binding = 3)
uniform sampler s_src_depth;

layout(location = 0) in vec2 uv;

layout (std140, set = 2, binding = 4)
uniform u_locals {
    mat4 all_mat_inv;
};

layout(location = 0) out vec4 tgt_color;

vec3 wpos_at(vec2 uv) {
    uvec2 sz = textureSize(sampler2D(t_src_depth, s_src_depth), 0);
    float buf_depth = texelFetch(sampler2D(t_src_depth, s_src_depth), clamp(ivec2(uv * sz), ivec2(0), ivec2(sz) - 1), 0).x;
    //float buf_depth = texture(sampler2D(t_src_depth, s_src_depth), uv).x;
    vec4 clip_space = vec4((uv * 2.0 - 1.0) * vec2(1, -1), buf_depth, 1.0);
    vec4 view_space = all_mat_inv * clip_space;
    view_space /= view_space.w;
    if (buf_depth == 0.0) {
        vec3 direction = normalize(view_space.xyz);
        return direction.xyz * 524288.0625 + cam_pos.xyz;
    } else {
        return view_space.xyz;
    }
}

float depth_at(vec2 uv) {
    uvec2 sz = textureSize(sampler2D(t_src_depth, s_src_depth), 0);
    float buf_depth = texelFetch(sampler2D(t_src_depth, s_src_depth), clamp(ivec2(uv * sz), ivec2(0), ivec2(sz) - 1), 0).x;
    if (buf_depth == 0.0) {
        return 524288.0;
    } else {
        vec4 clip_space = vec4((uv * 2.0 - 1.0) * vec2(1, -1), buf_depth, 1.0);
        vec4 view_space = all_mat_inv * clip_space;
        view_space /= view_space.w;
        return -(view_mat * view_space).z;
    }
}

void main() {
    vec4 color = texture(sampler2D(t_src_color, s_src_color), uv);

    #ifdef EXPERIMENTAL_BAREMINIMUM
        tgt_color = vec4(color.rgb, 1);
        return;
    #endif

    vec3 wpos = wpos_at(uv);
    float dist = distance(wpos, cam_pos.xyz);
    vec3 dir = (wpos - cam_pos.xyz) / dist;

    // Apply clouds
    float cloud_blend = 1.0;
    if (color.a < 1.0 && medium.x != MEDIUM_WATER) {
        cloud_blend = 1.0 - color.a;

        #ifdef EXPERIMENTAL_SCREENSPACEREFLECTIONS
            if (dir.z < 0.0) {
                #if (FLUID_MODE == FLUID_MODE_CHEAP)
                    vec2 nz = vec2(0);
                #else
                    vec2 nz = (vec2(
                        noise_3d(vec3((wpos.xy + focus_off.xy) * 0.1, tick.x * 0.2 + wpos.x * 0.01)).x,
                        noise_3d(vec3((wpos.yx + focus_off.yx) * 0.1, tick.x * 0.2 + wpos.y * 0.01)).x
                    ) - 0.5) * 5.0;
                #endif
                vec3 surf_norm = normalize(vec3(nz * 0.03 / (1.0 + dist * 0.1), 1));
                vec3 refl_dir = reflect(dir, surf_norm);

                vec4 clip = (all_mat * vec4(cam_pos.xyz + refl_dir, 1.0));
                vec2 new_uv = (clip.xy / max(clip.w, 0)) * 0.5 * vec2(1, -1) + 0.5;

                #ifdef EXPERIMENTAL_SCREENSPACEREFLECTIONSCASTING
                    vec3 ray_end = wpos + refl_dir * 5.0 * dist;
                    // Trace through the screen-space depth buffer to find the ray intersection
                    const int MAIN_ITERS = 64;
                    for (int i = 0; i < MAIN_ITERS; i ++) {
                        float t = float(i) / float(MAIN_ITERS);
                        // TODO: Trace in screen space, not world space
                        vec3 swpos = mix(wpos, ray_end, t);
                        vec3 svpos = (view_mat * vec4(swpos, 1)).xyz;
                        vec4 clippos = proj_mat * vec4(svpos, 1);
                        vec2 suv = (clippos.xy / clippos.w) * 0.5 * vec2(1, -1) + 0.5;
                        float d = -depth_at(suv);
                        if (d < svpos.z * 0.8 && d > svpos.z * 0.999) {
                            // Don't cast into water!
                            if (texture(sampler2D(t_src_color, s_src_color), suv).a >= 1.0) {
                                /* t -= 1.0 / float(MAIN_ITERS); */
                                // Do a bit of extra iteration to try to refine the estimate
                                const int ITERS = 8;
                                float diff = 1.0 / float(MAIN_ITERS);
                                for (int i = 0; i < ITERS; i ++) {
                                    vec3 swpos = mix(wpos, ray_end, t);
                                    svpos = (view_mat * vec4(swpos, 1)).xyz;
                                    vec4 clippos = proj_mat * vec4(svpos, 1);
                                    suv = (clippos.xy / clippos.w) * 0.5 * vec2(1, -1) + 0.5;
                                    float d = -depth_at(suv);
                                    t += ((d > svpos.z * 0.999) ? -1.0 : 1.0) * diff;
                                    diff *= 0.5;
                                }
                                // Small offset to push us into obscured territory
                                new_uv = suv - vec2(0, 0.001);
                                break;
                            }
                        }
                    }
                #endif

                new_uv = clamp(new_uv, vec2(0), vec2(1));

                vec3 new_wpos = wpos_at(new_uv);
                float new_dist = distance(new_wpos, cam_pos.xyz);
                float merge = min(
                    // Off-screen merge factor
                    clamp((1.0 - abs(new_uv.y - 0.5) * 2) * 3.0, 0, 1),
                    // Depth merge factor
                    clamp((new_dist - dist * 0.5) / (dist * 0.5), 0.0, 1.0)
                );

                if (merge > 0.0) {
                    vec3 new_col = texture(sampler2D(t_src_color, s_src_color), new_uv).rgb;
                    new_col = get_cloud_color(new_col.rgb, refl_dir, wpos, time_of_day.x, distance(new_wpos, wpos.xyz), 1.0);
                    color.rgb = mix(color.rgb, new_col, merge * (1.0 - color.a));
                    cloud_blend = 1;
                } else {
                    cloud_blend = 1;
                }
            } else {
        #else
            {
        #endif
            cloud_blend = 1;
            //dist = DIST_CAP;
        }
    }
    /* color.rgb = vec3(sin(depth_at(uv) * 3.14159 * 2) * 0.5 + 0.5); */
    color.rgb = mix(color.rgb, get_cloud_color(color.rgb, dir, cam_pos.xyz, time_of_day.x, dist, 1.0), cloud_blend);

    #if (CLOUD_MODE == CLOUD_MODE_NONE)
        color.rgb = apply_point_glow(cam_pos.xyz + focus_off.xyz, dir, dist, color.rgb);
    #else
        if (medium.x == MEDIUM_AIR && rain_density > 0.001) {
            vec3 cam_wpos = cam_pos.xyz + focus_off.xyz;

            vec3 adjusted_dir = (vec4(dir, 0) * rain_dir_mat).xyz;

            vec2 dir2d = adjusted_dir.xy;
            vec3 rorigin = cam_pos.xyz + focus_off.xyz + 0.5;
            vec3 rpos = vec3(0.0);
            float t = 0.0;
            const float PLANCK = 0.01;
            for (int i = 0; i < 14 /* log2(64) * 2 + 2 */; i ++) {
                float scale = min(pow(2, ceil(t / 2.0)), 32);
                vec2 deltas = (step(vec2(0), dir2d) - fract(rpos.xy / scale + 100.0)) / dir2d;
                float jump = max(min(deltas.x, deltas.y) * scale, PLANCK);
                t += jump;

                #if (CLOUD_MODE >= CLOUD_MODE_MEDIUM)
                    if (t >= 64.0) { break; }
                #else
                    if (t >= 16.0) { break; }
                #endif

                rpos = rorigin + adjusted_dir * t;

                vec2 diff = abs(round(rpos.xy) - rpos.xy);
                vec3 wall_pos = vec3((diff.x > diff.y) ? rpos.xy : rpos.yx, rpos.z + integrated_rain_vel);
                wall_pos.xz *= vec2(4, 0.3);
                wall_pos.z += hash_two(uvec2(wall_pos.xy + vec2(0, 0.5)));

                float depth_adjust = fract(hash_two(uvec2(wall_pos.xz) + 500u));
                float wpos_dist = t - jump * depth_adjust;
                vec3 wpos = cam_pos.xyz + dir * wpos_dist;

                if (wpos_dist > dist) { break; }
                if (length((fract(wall_pos.xz) - 0.5)) < 0.1 + pow(max(0.0, wpos_dist - (dist - 0.25)) / 0.25, 4.0) * 0.2) {
                    float density = rain_density * rain_occlusion_at(wpos);
                    if (fract(hash_two(uvec2(wall_pos.xz) + 1000u)) >= density) { continue; }

                    float alpha = 0.5 * clamp((wpos_dist - 1.0) * 0.5, 0.0, 1.0);
                    float light = dot(color.rgb, vec3(1)) + 0.05 + (get_sun_brightness() + get_moon_brightness()) * 0.2;
                    color.rgb = mix(color.rgb, vec3(0.3, 0.35, 0.5) * light, alpha);
                }
            }
        }
    #endif

    tgt_color = vec4(color.rgb, 1);
}
