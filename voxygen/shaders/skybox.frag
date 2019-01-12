#version 330 core

in vec3 f_pos;

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
	vec4 time;
};

out vec4 tgt_color;

void main() {
	tgt_color = vec4(f_pos, 1.0);
}
