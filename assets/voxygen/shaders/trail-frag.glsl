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

layout(location = 0) out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

const float FADE_DIST = 32.0;

void main() {
    vec3 trail_color = vec3(.55, .92, 1.0);
    float trail_alpha = 0.05;
    // Controls how much light affects alpha variation. TODO: Maybe a better name?
    float light_variable = 0.075;

    // Make less faint at day (relative to night) by adding light to alpha. Probably hacky but looks fine.
    // TODO: Trails should also eventually account for shadows, nearby lights, attenuation of sunlight in water, and block based lighting. Note: many of these will require alternative methods that don't require a normal.
    trail_alpha += get_sun_brightness() * light_variable;

    tgt_color = vec4(trail_color, trail_alpha);
}
