#version 420 core

#include <constants.glsl>

#define LIGHTING_TYPE (LIGHTING_TYPE_TRANSMISSION | LIGHTING_TYPE_REFLECTION)

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_SPECULAR

#if (FLUID_MODE == FLUID_MODE_CHEAP)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE == FLUID_MODE_SHINY)
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
    }
    color.rgb = mix(color.rgb, get_cloud_color(color.rgb, dir, cam_pos.xyz, time_of_day.x, dist, 1.0), cloud_blend);

    #if (CLOUD_MODE == CLOUD_MODE_NONE)
        color.rgb = apply_point_glow(cam_pos.xyz + focus_off.xyz, dir, dist, color.rgb);
    #elif (0 == 0)
        if (medium.x == MEDIUM_AIR && rain_density > 0.0) {
            vec3 cam_wpos = cam_pos.xyz + focus_off.xyz;

            vec3 adjusted_dir = (vec4(dir, 0) * rain_dir_mat).xyz;

            vec2 dir2d = adjusted_dir.xy;
            vec3 rorigin = cam_pos.xyz + focus_off.xyz + 0.5;
            vec3 rpos = vec3(0.0);
            float t = 0.0;
            for (int i = 0; i < 8; i ++) {
                const float PLANCK = 0.01;
                float scale = min(pow(2, ceil(t / 3.0)), 32);
                vec2 deltas = (step(vec2(0), dir2d) - fract(rpos.xy / scale + 100.0)) / dir2d;
                float jump = max(min(deltas.x, deltas.y) * scale, PLANCK);
                t += jump;
                rpos = rorigin + adjusted_dir * t;

                vec2 diff = abs(round(rpos.xy) - rpos.xy);
                vec3 wall_pos = vec3((diff.x > diff.y) ? rpos.xy : rpos.yx, rpos.z + integrated_rain_vel);
                wall_pos.xz *= vec2(4, 0.3);
                wall_pos.z += hash(fract(vec4(floor(wall_pos.xy + vec2(0, 0.5)), 1000, 0) * 0.1));

                float depth_adjust = abs(hash(vec4(floor(wall_pos.xyz), 2000)));
                float wpos_dist = t - jump * depth_adjust;
                vec3 wpos = cam_pos.xyz + dir * wpos_dist;

                float density = rain_density * 3.0 * rain_occlusion_at(wpos);
                if (density < 0.001 || fract(hash(vec4(floor(wall_pos.xyz), 0))) > density) { continue; }

                if (wpos_dist > dist) { break; }
                if (length((fract(wall_pos.xz) - 0.5)) < 0.1 + pow(max(0.0, wpos_dist - (dist - 0.25)) / 0.25, 4.0) * 0.2) {
                    float alpha = 0.5 * clamp((wpos_dist - 1.0) * 0.5, 0.0, 1.0);
                    float light = sqrt(dot(color.rgb, vec3(1))) + (get_sun_brightness() + get_moon_brightness()) * 0.01;
                    color.rgb = mix(color.rgb, vec3(0.2, 0.3, 0.5) * light, alpha);
                }
            }
        }
    #else
        vec3 old_color = color.rgb;

        // normalized direction from the camera position to the fragment in world, transformed by the relative rain direction
        vec3 adjusted_dir = (vec4(dir, 0) * rain_dir_mat).xyz;

        // stretch z values as they move away from 0
        float z = (-1 / (abs(adjusted_dir.z) - 1) - 1) * sign(adjusted_dir.z);
        // normalize xy to get a 2d direction
        vec2 dir_2d = normalize(adjusted_dir.xy);
        // sort of map cylinder around the camera to 2d grid
        vec2 view_pos = vec2(atan2(dir_2d.x, dir_2d.y), z);

        // compute camera position in the world
        vec3 cam_wpos = cam_pos.xyz + focus_off.xyz;

        // Rain density is now only based on the cameras current position.
        // This could be affected by a setting where rain_density_at is instead
        // called each iteration of the loop. With the current implementation
        // of rain_dir this has issues with being in a place where it doesn't rain
        // and seeing rain.
        float rain_density = rain_density * 1.0;
        if (medium.x == MEDIUM_AIR && rain_density > 0.0) {
            float rain_dist = 50.0;
            #if (CLOUD_MODE <= CLOUD_MODE_LOW)
                const int iterations = 2;
            #else
                const int iterations = 4;
            #endif

            for (int i = 0; i < iterations; i ++) {
                float old_rain_dist = rain_dist;
                rain_dist *= 0.3 / 4.0 * iterations;

                vec2 drop_density = vec2(30, 1);

                vec2 rain_pos = (view_pos * rain_dist);
                rain_pos.y += integrated_rain_vel;

                vec2 cell = floor(rain_pos * drop_density) / drop_density;

                float drop_depth = mix(
                    old_rain_dist,
                    rain_dist,
                    fract(hash(fract(vec4(cell, rain_dist, 0) * 0.1)))
                );

                float dist_to_rain = drop_depth / length(dir.xy);
                vec3 rpos = dir * dist_to_rain;
                if (dist < dist_to_rain || cam_wpos.z + rpos.z > CLOUD_AVG_ALT) {
                    continue;
                }

                if (dot(rpos * vec3(1, 1, 0.5), rpos) < 1.0) {
                    break;
                }
                float rain_density = 10.0 * rain_density * floor(rain_occlusion_at(cam_pos.xyz + rpos.xyz));

                if (rain_density < 0.001 || fract(hash(fract(vec4(cell, rain_dist, 0) * 0.01))) > rain_density) {
                    continue;
                }
                vec2 near_drop = cell + (vec2(0.5) + (vec2(hash(vec4(cell, 0, 0)), 0.5) - 0.5) * vec2(2, 0)) / drop_density;

                vec2 drop_size = vec2(0.0008, 0.03);
                float avg_alpha = (drop_size.x * drop_size.y) * 10 / 1;
                float alpha = sign(max(1 - length((rain_pos - near_drop) / drop_size * 0.1), 0));
                float light = sqrt(dot(old_color, vec3(1))) + (get_sun_brightness() + get_moon_brightness()) * 0.01;
                color.rgb = mix(color.rgb, vec3(0.3, 0.4, 0.5) * light, mix(avg_alpha, alpha, min(1000 / dist_to_rain, 1)) * 0.25);
            }
        }
    #endif

    tgt_color = vec4(color.rgb, 1);
}
