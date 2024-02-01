#version 440 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#if (FLUID_MODE == FLUID_MODE_LOW)
    #define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE >= FLUID_MODE_MEDIUM)
    #define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>

layout(location = 0) in vec3 f_pos;
layout(location = 1) in vec3 f_norm;
layout(location = 2) in vec4 f_col;
layout(location = 3) in vec3 model_pos;
layout(location = 4) flat in uint f_flags;

const uint FLAG_SNOW_COVERED = 1;
const uint FLAG_IS_BUILDING = 2;
const uint FLAG_IS_GIANT_TREE = 4;

layout(location = 0) out vec4 tgt_color;
layout(location = 1) out uvec4 tgt_mat;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

const float FADE_DIST = 32.0;

void main() {
    #ifdef EXPERIMENTAL_BAREMINIMUM
        tgt_color = vec4(simple_lighting(f_pos.xyz, f_col.rgb, 1.0), 1);
        return;
    #endif

    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    vec3 view_dir = -cam_to_frag;

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE >= FLUID_MODE_MEDIUM)
    float f_alt = alt_at(f_pos.xy);
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_LOW)
    float f_alt = f_pos.z;
#endif

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP)
    vec4 f_shadow = textureMaybeBicubic(t_horizon, s_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#elif (SHADOW_MODE == SHADOW_MODE_NONE)
    float sun_shade_frac = 1.0;
#endif
    float moon_shade_frac = 1.0;

    DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, f_pos);
    DirectionalLight moon_info = get_moon_info(moon_dir, moon_shade_frac);

    vec3 surf_color = f_col.rgb;
    float alpha = 1.0;
    const float n2 = 1.5;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float R_s = (f_pos.z < f_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);

    // Tree trunks
    if ((f_flags & FLAG_IS_GIANT_TREE) > 0u) {
        if (dot(abs(model_pos.xyz) * vec3(1.0, 1.0, 2.0), vec3(1)) < 430.0) { surf_color = vec3(0.05, 0.02, 0.0); }
    } else {
        if (model_pos.z < 25.0 && dot(abs(model_pos.xy), vec2(1)) < 6.0) { surf_color = vec3(0.05, 0.02, 0.0); }
    }

    vec3 voxel_norm = f_norm;
    float my_alt = f_pos.z + focus_off.z;
    float f_ao = 1.0;
    const float VOXELIZE_DIST = 2000;
    float voxelize_factor = clamp(1.0 - (distance(focus_pos.xy, f_pos.xy) - view_distance.x) * (1.0 / VOXELIZE_DIST), 0, 1.0);
    vec3 cam_dir = cam_to_frag;
    #ifdef EXPERIMENTAL_NOLODVOXELS
        vec3 side_norm = normalize(vec3(f_norm.xy, 0));
        vec3 top_norm = vec3(0, 0, 1);
        voxel_norm = normalize(mix(side_norm, top_norm, cam_dir.z));
    #else
        float t = -1.5;
        while (t < 1.5) {
            vec3 deltas = (step(vec3(0), -cam_dir) - fract(f_pos - cam_dir * t)) / -cam_dir;
            float m = min(min(deltas.x, deltas.y), deltas.z);

            t += max(m, 0.01);

            vec3 block_pos = floor(f_pos - cam_dir * t) + 0.5;
            if (dot(block_pos - f_pos, -f_norm) < 0.0) {
                vec3 to_center = abs(block_pos - (f_pos - cam_dir * t));
                voxel_norm = step(max(max(to_center.x, to_center.y), to_center.z), to_center) * sign(-cam_dir);
                voxel_norm = mix(f_norm, voxel_norm, voxelize_factor);
                surf_color *= mix(0.65, 1.0, hash_three(uvec3(block_pos + focus_off.xyz)));
                f_ao = mix(1.0, clamp(1.0 + t, 0.2, 1.0), voxelize_factor);
                break;
            }
        }
    #endif

    vec3 emitted_light, reflected_light;

    // To account for prior saturation.
    float max_light = 0.0;

    vec3 cam_attenuation = vec3(1);
    float fluid_alt = max(f_pos.z + 1, floor(f_alt + 1));
    vec3 mu = medium.x == MEDIUM_WATER ? MU_WATER : vec3(0.0);

    max_light += get_sun_diffuse2(sun_info, moon_info, voxel_norm, view_dir, f_pos, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, voxel_norm, 1.0, emitted_light, reflected_light);

    emitted_light *= f_ao;
    reflected_light *= f_ao;

    vec3 glow = vec3(0);
    if ((f_flags & FLAG_IS_BUILDING) > 0u && abs(f_norm.z) < 0.1) {
        ivec3 wpos = ivec3((f_pos.xyz + focus_off.xyz) * 0.2);
        if (((wpos.x & wpos.y & wpos.z) & 1) == 1) {
            glow += vec3(1, 0.7, 0.3) * 2;
        } else {
            reflected_light += vec3(1, 0.7, 0.3) * 0.9;
        }
    }

    vec3 side_color = surf_color;
    vec3 top_color = surf_color;
    if ((f_flags & FLAG_SNOW_COVERED) > 0u && f_norm.z > 0.0) {
        side_color = mix(side_color, vec3(0.5, 0.6, 1.0), f_norm.z);
        top_color = mix(top_color, surf_color * 0.3, 0.5 + f_norm.z * 0.5);
    }
    surf_color = mix(side_color, top_color, pow(fract(model_pos.z * 0.1), 2.0));

    surf_color = illuminate(max_light, view_dir, surf_color * emitted_light + glow, surf_color * reflected_light);

    tgt_color = vec4(surf_color, 1.0);
    tgt_mat = uvec4(uvec3((f_norm + 1.0) * 127.0), MAT_LOD);
}
