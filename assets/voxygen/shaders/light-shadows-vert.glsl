#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#if (FLUID_MODE == FLUID_MODE_CHEAP)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE == FLUID_MODE_SHINY)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

// Currently, we only need globals for the all_mat matrix.
#include <globals.glsl>

/* Accurate packed shadow maps for many lights at once!
 *
 * Ideally, we would just write to a bitmask...
 *
 * */

in uint v_pos_norm;
in uint v_col_light;

// Light projection matrices.
layout (std140)
uniform u_locals {
    vec3 model_offs;
	float load_time;
};

// out vec4 shadowMapCoord;

const int EXTRA_NEG_Z = 32768;

void main() {
	vec3 f_chunk_pos = vec3(ivec3((uvec3(v_pos_norm) >> uvec3(0, 6, 12)) & uvec3(0x3Fu, 0x3Fu, 0xFFFFu)) - ivec3(0, 0, EXTRA_NEG_Z));
	// f_pos = f_chunk_pos + model_offs;
	// f_pos = v_pos;
	vec3 f_pos = f_chunk_pos + model_offs;

	gl_Position = /*all_mat * */vec4(f_pos, 1.0);
    // shadowMapCoord = lights[gl_InstanceID].light_pos * gl_Vertex;
    // vec4(v_pos, 0.0, 1.0);
}
