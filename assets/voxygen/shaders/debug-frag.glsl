#version 440 core

#define HAS_SHADOW_MAPS

#include <constants.glsl>
#include <globals.glsl>
#include <srgb.glsl>
#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

layout (location = 0)
in vec4 f_color;
layout (location = 1)
in vec3 f_pos;
layout (location = 2)
in vec3 f_norm;

layout (std140, set = 2, binding = 0)
uniform u_locals {
    vec4 w_pos;
    vec4 w_color;
};

layout(set = 2, binding = 0)
uniform texture2D t_col_light;
layout(set = 2, binding = 1)
uniform sampler s_col_light;

layout(location = 0) out vec4 tgt_color;
layout(location = 1) out uvec4 tgt_mat;

void main() {
    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    vec3 view_dir = -cam_to_frag;

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE == FLUID_MODE_SHINY)
    float f_alt = alt_at(f_pos.xy);
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_CHEAP)
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

    vec3 surf_color = f_color.xyz;
    float alpha = 1.0;
    const float n2 = 1.5;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float R_s = (f_pos.z < f_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(0.8);
    vec3 k_s = vec3(R_s);
    float max_light = 0.0;
    vec3 cam_attenuation = vec3(1);
    float fluid_alt = max(f_pos.z + 1, floor(f_alt + 1));
    vec3 mu = medium.x == MEDIUM_WATER ? MU_WATER : vec3(0.0);
    vec3 emitted_light = vec3(1);
    vec3 reflected_light = vec3(1);

    max_light += get_sun_diffuse2(sun_info, moon_info, f_norm, view_dir, f_pos, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    max_light += lights_at(f_pos, f_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    float point_shadow = shadow_at(f_pos, f_norm);
    reflected_light *= point_shadow;
    emitted_light *= point_shadow;

    surf_color = illuminate(max_light, view_dir, surf_color * emitted_light, surf_color * reflected_light * 1.0);

    tgt_color = vec4(surf_color, f_color.a);
    tgt_mat = uvec4(uvec3((f_norm + 1.0) * 127.0), MAT_FIGURE);
}
