#version 330 core

in vec3 v_pos;
in vec3 v_col;
in uint v_bone;

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

out vec3 f_pos;

void main() {
	f_pos = v_pos;

	gl_Position =
		proj_mat *
		view_mat *
		vec4(0.5 * v_pos + cam_pos.xyz, 1);
}
