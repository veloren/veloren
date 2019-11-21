#version 330 core

#include <globals.glsl>
#include <srgb.glsl>

uniform sampler2D t_noise;

in vec2 v_pos;

layout (std140)
uniform u_locals {
	vec4 nul;
};

out vec3 f_pos;
out vec3 f_norm;
out vec3 f_col;
out float f_light;

void main() {
	vec2 pos = v_pos * 1000.0;

	f_pos = vec3(pos, texture(t_noise, pos * 0.001).x * 1000.0);

	//f_pos.z -= 25.0 * pow(distance(focus_pos.xy, f_pos.xy) / view_distance.x, 20.0);

	f_col = vec3(0.5, 1.0, 0.3);

	f_light = 1.0;

	f_norm = vec3(0, 0, 1);

	gl_Position =
		proj_mat *
		view_mat *
		vec4(f_pos, 1);
	gl_Position.z = 1.0 / (1.0 - gl_Position.z - 10.0);
}
