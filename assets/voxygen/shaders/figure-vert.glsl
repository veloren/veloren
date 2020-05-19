#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <lod.glsl>

in uint v_pos_norm;
in vec3 v_norm;
in uint v_col;
in uint v_ao_bone;

layout (std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
	// bit 0 - is player
	// bit 1-31 - unused
	int flags;
};

struct BoneData {
	mat4 bone_mat;
};

layout (std140)
uniform u_bones {
	// Warning: might not actually be 16 elements long. Don't index out of bounds!
	BoneData bones[16];
};

out vec3 f_pos;
out vec3 f_col;
out float f_ao;
flat out vec3 f_norm;
// out float f_alt;
// out vec4 f_shadow;

void main() {
	// Pre-calculate bone matrix
	uint bone_idx = (v_ao_bone >> 2) & 0x3Fu;
	mat4 combined_mat = model_mat * bones[bone_idx].bone_mat;

	vec3 pos = (vec3((uvec3(v_pos_norm) >> uvec3(0, 9, 18)) & uvec3(0x1FFu)) - 256.0) / 2.0;

	f_pos = (
		combined_mat *
		vec4(pos, 1)).xyz;

	f_col = srgb_to_linear(vec3((uvec3(v_col) >> uvec3(0, 8, 16)) & uvec3(0xFFu)) / 255.0);

	f_ao = float(v_ao_bone & 0x3u) / 4.0;

	// First 3 normals are negative, next 3 are positive
	vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));
	vec3 norm = normals[(v_pos_norm >> 29) & 0x7u];

	// Calculate normal here rather than for each pixel in the fragment shader
	f_norm = normalize((
		combined_mat *
		vec4(norm, 0.0)
	).xyz);

    // Also precalculate shadow texture and estimated terrain altitude.
    // f_alt = alt_at(f_pos.xy);
    // f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));

	gl_Position = all_mat * vec4(f_pos, 1);
    // gl_Position.z = -gl_Position.z / gl_Position.w;
	// gl_Position.z = -gl_Position.z / 100.0;
	gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
