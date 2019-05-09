#version 330 core

in vec3 v_pos;
in vec3 v_norm;
in vec3 v_col;

layout (std140)
uniform u_locals {
	vec3 model_offs;
};

layout (std140)
uniform u_globals {
	mat4 view_mat;
	mat4 proj_mat;
	vec4 cam_pos;
	vec4 focus_pos;
	vec4 view_distance;
	vec4 time_of_day;
	vec4 tick;
	vec4 screen_res;
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
		vec4(v_pos + model_offs, 1);
}
