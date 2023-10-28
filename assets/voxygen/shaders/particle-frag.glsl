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
layout(location = 2) in vec4 f_col;
layout(location = 3) in float f_reflect;

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

    vec3 k_a = vec3(1.0) * f_reflect;
    vec3 k_d = vec3(1.0) * f_reflect;
    vec3 k_s = vec3(R_s) * f_reflect;

    vec3 emitted_light, reflected_light;

    // This is a bit of a hack. Because we can't find the volumetric lighting of each particle (they don't talk to the
    // CPU) we need to some how find an approximation of how much the sun is blocked. We do this by fading out the sun
    // as the particle moves underground. This isn't perfect, but it does at least mean that particles don't look like
    // they're exposed to the sun when in dungeons
    const float SUN_FADEOUT_DIST = 20.0;
    sun_info.block *= clamp((f_pos.z - f_alt) / SUN_FADEOUT_DIST + 1, 0, 1);

    // To account for prior saturation.
    float max_light = 0.0;

    vec3 cam_attenuation = vec3(1);
    float fluid_alt = max(f_pos.z + 1, floor(f_alt + 1));
    vec3 mu = medium.x == MEDIUM_WATER ? MU_WATER : vec3(0.0);
    #if (FLUID_MODE >= FLUID_MODE_MEDIUM)
        cam_attenuation =
            medium.x == MEDIUM_WATER ? compute_attenuation_point(cam_pos.xyz, view_dir, MU_WATER, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/f_pos)
            : compute_attenuation_point(f_pos, -view_dir, vec3(0), fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/cam_pos.xyz);
    #endif

    max_light += get_sun_diffuse2(sun_info, moon_info, f_norm, view_dir, f_pos, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    max_light += lights_at(f_pos, f_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    float point_shadow = shadow_at(f_pos, f_norm);
    reflected_light *= point_shadow;
    emitted_light *= point_shadow;

    // Allow particles to glow at night
    // TODO: Not this
    emitted_light += max(f_col.rgb - 1.0, vec3(0));

    surf_color = illuminate(max_light, view_dir, surf_color * emitted_light, surf_color * reflected_light * f_reflect);

    // Temporarily disable particle transparency to avoid artifacts
    tgt_color = vec4(surf_color, 1.0 /*f_col.a*/);
    tgt_mat = uvec4(uvec3((f_norm + 1.0) * 127.0), MAT_BLOCK);
}
