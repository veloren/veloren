#version 330 core

#include <globals.glsl>

in vec3 v_pos;
in vec3 v_norm;
in vec3 v_col;
in uint v_bone_idx;

layout (std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
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
flat out vec3 f_norm;

void main() {
	// Pre-calculate bone matrix
	mat4 combined_mat = model_mat * bones[v_bone_idx].bone_mat;

	f_pos = (
		combined_mat *
		vec4(v_pos, 1)).xyz;

	f_col = v_col;

	// Calculate normal here rather than for each pixel in the fragment shader
	f_norm = normalize((
		combined_mat *
		vec4(v_norm, 0.0)
	).xyz);

	gl_Position = all_mat * vec4(f_pos, 1);
	gl_Position.z = 1.0 / (1.0 - gl_Position.z - 10.0);
}
