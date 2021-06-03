#version 420 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#define HAS_SHADOW_MAPS

#include <globals.glsl>

layout(location = 0) in vec3 f_pos;
layout(location = 1) flat in vec3 f_norm;
layout(location = 2) flat in float f_select;
layout(location = 3) in vec2 f_uv_pos;
layout(location = 4) in vec2 f_inst_light;

layout(set = 3, binding = 0)
uniform texture2D t_col_light;
layout(set = 3, binding = 1)
uniform sampler s_col_light;

layout(location = 0) out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

const float FADE_DIST = 32.0;

void main() {
    float f_ao, f_glow;
    vec3 f_col = greedy_extract_col_light_glow(t_col_light, s_col_light, f_uv_pos, f_ao, f_glow);

    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    vec3 view_dir = -cam_to_frag;

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE == FLUID_MODE_SHINY)
    float f_alt = alt_at(f_pos.xy);
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_CHEAP)
    float f_alt = f_pos.z;
#endif

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP)
    vec4 f_shadow = textureBicubic(t_horizon, s_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#elif (SHADOW_MODE == SHADOW_MODE_NONE)
    float sun_shade_frac = 1.0;
#endif
    float moon_shade_frac = 1.0;

    float point_shadow = shadow_at(f_pos, f_norm);
    DirectionalLight sun_info = get_sun_info(sun_dir, point_shadow * sun_shade_frac, f_pos);
    DirectionalLight moon_info = get_moon_info(moon_dir, point_shadow * moon_shade_frac);

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

    vec3 emitted_light, reflected_light;

    // Make voxel shadows block the sun and moon
    sun_info.block = f_inst_light.x;
    moon_info.block = f_inst_light.x;

    float max_light = 0.0;
    max_light += get_sun_diffuse2(sun_info, moon_info, f_norm, view_dir, k_a, k_d, k_s, alpha, emitted_light, reflected_light);

    max_light += lights_at(f_pos, f_norm, view_dir, k_a, k_d, k_s, alpha, emitted_light, reflected_light);

    vec3 glow = pow(f_inst_light.y, 3) * 4 * glow_light(f_pos);
    emitted_light += glow;

    float ao = f_ao;
    emitted_light *= ao;
    reflected_light *= ao;

    surf_color = illuminate(max_light, view_dir, surf_color * emitted_light, surf_color * reflected_light);

    surf_color += f_select * (surf_color + 0.1) * vec3(0.15, 0.15, 0.15);

    tgt_color = vec4(surf_color, 1.0 - clamp((distance(focus_pos.xy, f_pos.xy) - (sprite_render_distance - FADE_DIST)) / FADE_DIST, 0, 1));
    //tgt_color = vec4(-f_norm, 1.0);
}
