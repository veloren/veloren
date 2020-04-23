#version 330 core

#include <globals.glsl>

in vec3 v_pos;
in vec3 v_norm;
in vec3 v_col;
in float v_ao;
in uint v_bone_idx;

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
	BoneData bones[16];
};

out vec3 f_pos;
out vec3 f_col;
out float f_ao;
flat out vec3 f_norm;

void main() {
	// Pre-calculate bone matrix
	mat4 combined_mat = model_mat * bones[v_bone_idx].bone_mat;

	f_pos = (
		combined_mat *
		vec4(v_pos, 1)).xyz;

	f_col = v_col;

	f_ao = v_ao;

	// Calculate normal here rather than for each pixel in the fragment shader
	f_norm = normalize((
		combined_mat *
		vec4(v_norm, 0.0)
	).xyz);

	gl_Position = all_mat * vec4(f_pos, 1);
	gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
