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
    mat4 proj_mat_inv;
    mat4 view_mat_inv;
};

layout(location = 0) out vec4 tgt_color;

vec3 wpos_at(vec2 uv) {
    float buf_depth = texture(sampler2D(t_src_depth, s_src_depth), uv).x;
    mat4 inv = view_mat_inv * proj_mat_inv;//inverse(all_mat);
    vec4 clip_space = vec4((uv * 2.0 - 1.0) * vec2(1, -1), buf_depth, 1.0);
    vec4 view_space = inv * clip_space;
    view_space /= view_space.w;
    if (buf_depth == 0.0) {
        vec3 direction = normalize(view_space.xyz);
        return direction.xyz * 524288.0625 + cam_pos.xyz;
    } else {
        return view_space.xyz;
    }
}

mat4 spin_in_axis(vec3 axis, float angle)
{
    axis = normalize(axis);
    float s = sin(angle);
    float c = cos(angle);
    float oc = 1.0 - c;

    return mat4(oc * axis.x * axis.x + c,  oc * axis.x * axis.y - axis.z * s, oc * axis.z * axis.x + axis.y * s, 0,
        oc * axis.x * axis.y + axis.z * s, oc * axis.y * axis.y + c,          oc * axis.y * axis.z - axis.x * s, 0,
        oc * axis.z * axis.x - axis.y * s, oc * axis.y * axis.z + axis.x * s, oc * axis.z * axis.z + c,          0,
        0,                                 0,                                 0,                                 1);
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
    #endif

    #ifdef EXPERIMENTAL_RAIN
        vec3 old_color = color.rgb;

        // If this value is changed also change it in voxygen/src/scene/mod.rs
        float fall_rate = 70.0;
        dir.xy += wind_vel * dir.z / fall_rate;
        dir = normalize(dir);

        float z = (-1 / (abs(dir.z) - 1) - 1) * sign(dir.z);
        vec2 dir_2d = normalize(dir.xy);
        vec2 view_pos = vec2(atan2(dir_2d.x, dir_2d.y), z);

        vec3 cam_wpos = cam_pos.xyz + focus_off.xyz;
        float rain_dist = 250.0;
        for (int i = 0; i < 7; i ++) {
            float old_rain_dist = rain_dist;
            rain_dist *= 0.3;

            vec2 drop_density = vec2(30, 1);
            vec2 drop_size = vec2(0.0008, 0.05);

            vec2 rain_pos = (view_pos * rain_dist);
            rain_pos += vec2(0, tick.x * fall_rate + cam_wpos.z);

            vec2 cell = floor(rain_pos * drop_density) / drop_density;

            float drop_depth = mix(
                old_rain_dist,
                rain_dist,
                fract(hash(fract(vec4(cell, rain_dist, 0) * 0.1)))
            );
            vec3 rpos = vec3(vec2(dir_2d), view_pos.y) * drop_depth;
            float dist_to_rain = length(rpos);
            if (dist < dist_to_rain || cam_wpos.z + rpos.z > CLOUD_AVG_ALT) {
                continue;
            }

            if (dot(rpos * vec3(1, 1, 0.5), rpos) < 1.0) {
                break;
            }
            float rain_density = rain_density_at(cam_wpos.xy + rpos.xy) * rain_occlusion_at(cam_pos.xyz + rpos.xyz) * 10.0;

            if (fract(hash(fract(vec4(cell, rain_dist, 0) * 0.01))) > rain_density) {
                continue;
            }
            vec2 near_drop = cell + (vec2(0.5) + (vec2(hash(vec4(cell, 0, 0)), 0.5) - 0.5) * vec2(2, 0)) / drop_density;

            float avg_alpha = (drop_size.x * drop_size.y) * 10 / 1;
            float alpha = sign(max(1 - length((rain_pos - near_drop) / drop_size * 0.1), 0));
            float light = sqrt(dot(old_color, vec3(1))) + (get_sun_brightness() + get_moon_brightness()) * 0.01;
            color.rgb = mix(color.rgb, vec3(0.3, 0.4, 0.5) * light, mix(avg_alpha, alpha, min(1000 / dist_to_rain, 1)) * 0.25);
        }
    #endif

    tgt_color = vec4(color.rgb, 1);
}
