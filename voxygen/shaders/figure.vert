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
out vec3 f_norm;
out vec3 f_col;
flat out uint f_bone_idx;

void main() {
	f_pos = (model_mat *
		bones[v_bone_idx].bone_mat *
		vec4(v_pos, 1)).xyz;
	f_norm = v_norm;
	f_col = v_col;
	f_bone_idx = v_bone_idx;

	gl_Position = proj_mat * view_mat * vec4(f_pos, 1);
}
