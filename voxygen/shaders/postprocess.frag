#version 330 core

uniform sampler2D src_color;

in vec2 f_pos;

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
};

out vec4 tgt_color;

void main() {
	// Uncomment to invert colors
	//tgt_color = vec4(vec3(1.0, 1.0, 1.0) - texture2D(src_color, (f_pos + 1.0) / 2.0).xyz, 1.0);
	tgt_color = texture2D(src_color, (f_pos + 1.0) / 2.0);
	
}
