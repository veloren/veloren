#version 440 core
// #extension ARB_texture_storage : enable

#define FIGURE_SHADER

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
layout(location = 1) in uint v_atlas_pos;
// in uint v_col_light;
// in vec4 v_pos;

layout (std140, set = 1, binding = 0)
uniform u_locals {
    mat4 model_mat;
    vec4 highlight_col;
    vec4 model_light;
    vec4 model_glow;
    ivec4 atlas_offs;
    vec3 model_pos;
    // bit 0 - is player
    // bit 1-31 - unused
    int flags;
};

struct BoneData {
    mat4 bone_mat;
    mat4 normals_mat;
};

layout (std140, set = 1, binding = 1)
uniform u_bones {
    // Warning: might not actually be 16 elements long. Don't index out of bounds!
    BoneData bones[16];
};

// out vec4 shadowMapCoord;

void main() {
    uint bone_idx = (v_pos_norm >> 27) & 0xFu;
    vec3 pos = (vec3((uvec3(v_pos_norm) >> uvec3(0, 9, 18)) & uvec3(0x1FFu)) - 256.0) / 2.0;

    vec3 f_pos = (
        bones[bone_idx].bone_mat *
        vec4(pos, 1.0)
    ).xyz + (model_pos - focus_off.xyz/* + vec3(0.0, 0.0, 0.0001)*/);

    gl_Position = shadowMatrices * vec4(f_pos, 1.0);
}
