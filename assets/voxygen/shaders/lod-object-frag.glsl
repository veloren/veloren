#version 420 core

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
layout(location = 4) in float snow_cover;

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
    vec4 f_shadow = textureBicubic(t_horizon, s_horizon, pos_to_tex(f_pos.xy));
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

    vec3 my_norm = vec3(f_norm.xy, abs(f_norm.z));
    vec3 voxel_norm;
    float my_alt = f_pos.z + focus_off.z;
    float f_ao = 1.0;
    const float VOXELIZE_DIST = 2000;
    float voxelize_factor = clamp(1.0 - (distance(focus_pos.xy, f_pos.xy) - view_distance.x) / VOXELIZE_DIST, 0, 0.65);
    vec3 cam_dir = normalize(cam_pos.xyz - f_pos.xyz);
    vec3 side_norm = normalize(vec3(my_norm.xy, 0));
    vec3 top_norm = vec3(0, 0, 1);
    #ifdef EXPERIMENTAL_NOLODVOXELS
        f_ao = 1.0;
        voxel_norm = normalize(mix(side_norm, top_norm, cam_dir.z));
    #else
        float side_factor = 1.0 - my_norm.z;
        // min(dot(vec3(0, -sign(cam_dir.y), 0), -cam_dir), dot(vec3(-sign(cam_dir.x), 0, 0), -cam_dir))
        if (max(abs(my_norm.x), abs(my_norm.y)) < 0.01 || fract(my_alt) * clamp(dot(normalize(vec3(cam_dir.xy, 0)), side_norm), 0, 1) < cam_dir.z / my_norm.z) {
            f_ao *= mix(1.0, clamp(fract(my_alt) / length(my_norm.xy) + clamp(dot(side_norm, -cam_dir), 0, 1), 0, 1), voxelize_factor);
            voxel_norm = top_norm;
        } else {
            f_ao *= mix(1.0, clamp(pow(fract(my_alt), 0.5), 0, 1), voxelize_factor);

            if (fract(f_pos.x) * abs(my_norm.y / cam_dir.x) < fract(f_pos.y) * abs(my_norm.x / cam_dir.y)) {
                voxel_norm = vec3(sign(cam_dir.x), 0, 0);
            } else {
                voxel_norm = vec3(0, sign(cam_dir.y), 0);
            }
        }
        f_ao = min(f_ao, max(f_norm.z * 0.5 + 0.5, 0.0));
        voxel_norm = mix(my_norm, voxel_norm == vec3(0.0) ? f_norm : voxel_norm, voxelize_factor);
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

    vec3 side_color = mix(surf_color, vec3(0.5, 0.6, 1.0), snow_cover);
    vec3 top_color = mix(surf_color, surf_color * 0.3, 0.5 + snow_cover * 0.5);
    surf_color = mix(side_color, top_color, pow(fract(model_pos.z * 0.1), 2.0));

    surf_color = illuminate(max_light, view_dir, surf_color * emitted_light, surf_color * reflected_light);

    tgt_color = vec4(surf_color, 1.0);
    tgt_mat = uvec4(uvec3((f_norm + 1.0) * 127.0), MAT_LOD);
}
