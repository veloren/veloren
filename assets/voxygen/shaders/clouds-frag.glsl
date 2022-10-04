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
    float buf_depth = texture(sampler2D(t_src_depth, s_src_depth), uv).x;
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

vec3 dir_at(vec2 uv) {
    vec4 view_space = all_mat_inv * vec4((uv * 2.0 - 1.0) * vec2(1, -1), 0.0, 1.0);
    view_space /= view_space.w;
    return normalize(view_space.xyz);
}

float invlerp(float a, float b, float v) {
    return (v - a) / (b - a);
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
    if (color.a < 1.0) {
        cloud_blend = 1.0 - color.a;
        dist = DIST_CAP;
        //color.rgb = vec3(1, 0, 0);

        vec2 uv_refl = uv;
        vec3 wpos_refl = wpos;
        /* for (int i = 0; i < 10; i ++) { */
        /*     uv_refl.y -= 0.1; */
        /*     wpos_refl += reflect(dir, vec3(0, 0, 1)) * dist * 0.5; */
        /*     if (distance(wpos_at(uv_refl), cam_pos.xyz) < distance(wpos_refl, cam_pos.xyz)) { */
        /*         color.rgb = mix(color.rgb, texture(sampler2D(t_src_color, s_src_color), uv_refl).rgb, 0.9); */
        /*         break; */
        /*     } */
        /* } */

        if (dir.z < 0.0) {
            vec3 dir_mid = dir_at(vec2(0, 0));//normalize((all_mat_inv * vec4(0, 0, 0, 0)).xyz);
            vec3 dir_right = dir_at(vec2(1, 0));//normalize((all_mat_inv * vec4(1, 0, 0, 0)).xyz);
            vec3 dir_up = dir_at(vec2(0, 1));//normalize((all_mat_inv * vec4(0, -1, 0, 0)).xyz);
            vec3 surf_norm = normalize(vec3((vec2(noise_3d(vec3((wpos.xy + focus_off.xy) * 0.1, tick.x * 0.3)).x, noise_3d(vec3((wpos.xy + focus_off.xy).yx * 0.1, tick.x * 0.3)).x) - 0.5) * 0.03, 1));
            vec3 refl_dir = reflect(dir, surf_norm);

            float right = invlerp(dir_mid.x, dir_right.x, refl_dir.x);
            float up = invlerp(dir_mid.z, dir_up.z, refl_dir.z);

            vec2 new_uv = vec2(uv.x, up);

            float merge = clamp((1.0 - abs(new_uv.y - 0.5) * 2) * 3.5, 0, 0.75);

            //vec2 new_uv = uv * vec2(1, -1) + vec2(0, 1.1) / (1.0 + dist * 0.000001) + vec2(0, dir.z);
            color.rgb = mix(color.rgb, texture(sampler2D(t_src_color, s_src_color), new_uv).rgb, merge);
            wpos = wpos_at(new_uv);
            dist = distance(wpos, cam_pos.xyz);
            dir = (wpos - cam_pos.xyz) / dist;
            cloud_blend = min(merge * 2.0, 1.0);
        }
    }
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
