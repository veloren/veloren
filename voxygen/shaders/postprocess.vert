#version 330 core

in vec2 v_pos;

layout (std140)
uniform u_locals {
	vec4 nul;
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

out vec2 f_pos;

void main() {
	f_pos = v_pos;

	gl_Position = vec4(v_pos, 0.0, 1.0);
}
