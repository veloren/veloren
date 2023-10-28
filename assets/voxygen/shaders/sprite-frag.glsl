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

#define HAS_SHADOW_MAPS

#include <globals.glsl>

layout(location = 0) in vec3 f_pos;
layout(location = 1) flat in vec3 f_norm;
layout(location = 2) flat in float f_select;
layout(location = 3) in vec2 f_uv_pos;
layout(location = 4) in vec2 f_inst_light;

layout(set = 2, binding = 0)
uniform texture2D t_col_light;
layout(set = 2, binding = 1)
uniform sampler s_col_light;

layout(location = 0) out vec4 tgt_color;
layout(location = 1) out uvec4 tgt_mat;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

const float FADE_DIST = 32.0;

void main() {
    float f_ao;
    uint material = 0xFFu;
    vec3 f_col = greedy_extract_col_light_figure(t_col_light, s_col_light, f_uv_pos, f_ao, material);

    #ifdef EXPERIMENTAL_BAREMINIMUM
        tgt_color = vec4(simple_lighting(f_pos.xyz, f_col, f_ao), 1);
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

    vec3 surf_color = f_col;
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

    vec3 emitted_light = vec3(1);
    vec3 reflected_light = vec3(1);

    // Make voxel shadows block the sun and moon
    sun_info.block = f_inst_light.x;
    moon_info.block = f_inst_light.x;

    float max_light = 0.0;

    vec3 cam_attenuation = vec3(1);
    float fluid_alt = max(f_pos.z + 1, floor(f_alt + 1));
    vec3 mu = medium.x == MEDIUM_WATER ? MU_WATER : vec3(0.0);
    #if (FLUID_MODE >= FLUID_MODE_MEDIUM)
        cam_attenuation =
            medium.x == MEDIUM_WATER ? compute_attenuation_point(cam_pos.xyz, view_dir, mu, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/f_pos)
            : compute_attenuation_point(f_pos, -view_dir, mu, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/cam_pos.xyz);
    #endif

    // Prevent the sky affecting light when underground
    float not_underground = clamp((f_pos.z - f_alt) / 128.0 + 1.0, 0.0, 1.0);

    max_light += get_sun_diffuse2(sun_info, moon_info, f_norm, view_dir, f_pos, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    emitted_light *= sun_info.block;
    reflected_light *= sun_info.block;

    max_light += lights_at(f_pos, f_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    // TODO: Hack to add a small amount of underground ambient light to the scene
    reflected_light += vec3(0.01, 0.02, 0.03) * (1.0 - not_underground);

    vec3 glow = pow(f_inst_light.y, 3) * 4 * glow_light(f_pos);
    emitted_light += glow * cam_attenuation;

    float ao = f_ao;
    reflected_light *= ao;
    emitted_light *= ao;

    float point_shadow = shadow_at(f_pos, f_norm);
    reflected_light *= point_shadow;
    emitted_light *= point_shadow;

    // Apply emissive glow
    // For now, just make glowing material light be the same colour as the surface
    // TODO: Add a way to control this better outside the shaders
    if ((material & (1u << 0u)) > 0u) {
        emitted_light += 20 * surf_color;
    }

    surf_color = illuminate(max_light, view_dir, surf_color * emitted_light, surf_color * reflected_light);

    surf_color += f_select * (surf_color + 0.1) * vec3(0.15, 0.15, 0.15);

    tgt_color = vec4(surf_color, 1.0 - clamp((distance(focus_pos.xy, f_pos.xy) - (sprite_render_distance - FADE_DIST)) / FADE_DIST, 0, 1));
    tgt_mat = uvec4(uvec3((f_norm + 1.0) * 127.0), MAT_FIGURE);
    //tgt_color = vec4(-f_norm, 1.0);
}
