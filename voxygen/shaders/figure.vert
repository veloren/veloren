#version 330 core

in vec3 v_pos;
in vec3 v_norm;
in vec3 v_col;
in uint v_bone_idx;

layout (std140)
uniform u_locals {
	mat4 model_mat;
};

layout (std140)
uniform u_globals {
	mat4 view_mat;
	mat4 proj_mat;
	vec4 cam_pos;
	vec4 focus_pos;
	vec4 view_distance;
	vec4 time_of_day;
	vec4 time;
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

void main() {
	f_pos = v_pos;
	f_norm = v_norm;
	f_col = v_col;

	gl_Position =
		proj_mat *
		view_mat *
		model_mat *
		bones[v_bone_idx].bone_mat *
		vec4(v_pos, 1);
}
