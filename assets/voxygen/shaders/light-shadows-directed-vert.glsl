#version 440 core
// #extension ARB_texture_storage : enable

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

// Currently, we only need globals for focus_off.
#include <globals.glsl>
// For shadow locals.
// #include <shadows.glsl>

layout (std140, set = 0, binding = 9)
uniform u_light_shadows {
    mat4 shadowMatrices;
    mat4 texture_mat;
};

/* Accurate packed shadow maps for many lights at once!
 *
 * Ideally, we would just write to a bitmask...
 *
 * */

layout(location = 0) in uint v_pos_norm;
// in uint v_col_light;
// in vec4 v_pos;
// layout(location = 1) in uint v_atlas_pos;

// Light projection matrices.
layout (std140, set = 1,  binding = 0)
uniform u_locals {
    mat4 model_mat;
    ivec4 atlas_offs;
    float load_time;
};

// out vec4 shadowMapCoord;

const float EXTRA_NEG_Z = 32768.0;

void main() {
    vec3 f_chunk_pos = vec3(v_pos_norm & 0x3Fu, (v_pos_norm >> 6) & 0x3Fu, float((v_pos_norm >> 12) & 0xFFFFu) - EXTRA_NEG_Z);
    vec3 f_pos = (model_mat * vec4(f_chunk_pos, 1.0)).xyz - focus_off.xyz;
    // f_pos = v_pos;

    gl_Position = /*all_mat * */shadowMatrices * vec4(f_pos/*, 1.0*/, /*float(((f_pos_norm >> 29) & 0x7u) ^ 0x1)*//*uintBitsToFloat(v_pos_norm)*/1.0);
    // gl_Position.z = -gl_Position.z;
    // gl_Position.z = clamp(gl_Position.z, -abs(gl_Position.w), abs(gl_Position.w));
    // shadowMapCoord = lights[gl_InstanceID].light_pos * gl_Vertex;
    // vec4(v_pos, 0.0, 1.0);
}
