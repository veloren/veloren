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
    //mat4 proj_mat_inv;
    //mat4 view_mat_inv;
    mat4 all_mat_inv;
};

layout(location = 0) out vec4 tgt_color;

// start
// 777 instructions with rain commented out
// 0.55 - 0.58 ms staring at high time area in sky in rain
// 0.48 ms staring at roughly open sky in rain 45 degree
// 0.35 ms staring at feet in rain

// precombine inversion matrix
// 683 instructions 
// 0.55 ms starting at high time arena in sky in rain
// 0.46 ms staring roughly open sky roughly 45 degree in rain
// 0.33 ms staring at feet in rain


vec3 wpos_at(vec2 uv) {
    float buf_depth = texture(sampler2D(t_src_depth, s_src_depth), uv).x;
    //mat4 inv = view_mat_inv * proj_mat_inv;//inverse(all_mat);
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

/*mat4 spin_in_axis(vec3 axis, float angle)
{
    axis = normalize(axis);
    float s = sin(angle);
    float c = cos(angle);
    float oc = 1.0 - c;

    return mat4(oc * axis.x * axis.x + c,  oc * axis.x * axis.y - axis.z * s, oc * axis.z * axis.x + axis.y * s, 0,
        oc * axis.x * axis.y + axis.z * s, oc * axis.y * axis.y + c,          oc * axis.y * axis.z - axis.x * s, 0,
        oc * axis.z * axis.x - axis.y * s, oc * axis.y * axis.z + axis.x * s, oc * axis.z * axis.z + c,          0,
        0,                                 0,                                 0,                                 1);
}*/

void main() {
    vec4 color = texture(sampler2D(t_src_color, s_src_color), uv);
    color.rgb *= 0.25;

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
   //color.rgb = mix(color.rgb, get_cloud_color(color.rgb, dir, cam_pos.xyz, time_of_day.x, dist, 1.0), cloud_blend);

    #if (CLOUD_MODE == CLOUD_MODE_NONE)
        color.rgb = apply_point_glow(cam_pos.xyz + focus_off.xyz, dir, dist, color.rgb);
    #else
        vec3 old_color = color.rgb;

        // 0.43 ms extra without this
        // 0.01 ms spent on rain_density_at?
        // 0.49 -> 0.13 (0.36 ms with full occupancy)
        //tgt_color = vec4(color.rgb, 1);
        //return;

        // normalized direction from the camera position to the fragment in world, transformed by the relative rain direction
        dir = (vec4(dir, 0) * rel_rain_dir_mat).xyz;

        // stretch z values far from 0
        float z = (-1 / (abs(dir.z) - 1) - 1) * sign(dir.z);
        // normalize xy to get a 2d direction
        vec2 dir_2d = normalize(dir.xy);
        // view_pos is the angle from x axis (except x and y are flipped, so the
        // angle 0 is looking along the y-axis)
        //
        // combined with stretched z position essentially we unroll a cylinder
        // around the z axis while stretching it to make the sections near the
        // origin larger in the Z direction
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
                int iterations = 2;
            #elif (CLOUD_MODE == CLOUD_MODE_MEDIUM)
                int iterations = 3;
            #else
                int iterations = 4;
            #endif

            for (int i = 0; i < iterations; i ++) {
                float old_rain_dist = rain_dist;
                rain_dist *= 0.3;

                vec2 drop_density = vec2(30, 1);

                vec2 rain_pos = (view_pos * rain_dist);
                rain_pos.y += integrated_rain_vel;

                vec2 cell = floor(rain_pos * drop_density) / drop_density;

                // For reference:
                //
                // float hash(vec4 p) {
                //     p = fract(p * 0.3183099 + 0.1) - fract(p + 23.22121);
                //     p *= 17.0;
                //     return (fract(p.x * p.y * (1.0 - p.z) * p.w * (p.x + p.y + p.z + p.w)) - 0.5) * 2.0;
                // }
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
                float rain_density = rain_density * rain_occlusion_at(cam_pos.xyz + rpos.xyz) * 10.0;

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
