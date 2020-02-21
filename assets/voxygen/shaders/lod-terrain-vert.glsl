#version 330 core

#include <globals.glsl>
#include <srgb.glsl>
#include <lod.glsl>

in vec2 v_pos;

layout (std140)
uniform u_locals {
	vec4 nul;
};

out vec3 f_pos;
out float f_light;

void main() {
	f_pos = lod_pos(v_pos + vec2(0, -v_pos.x * 0.5));

	f_pos.z -= 5.0 / pow(distance(focus_pos.xy, f_pos.xy) / (view_distance.x * 0.9), 20.0);

	f_light = 1.0;

	gl_Position =
		proj_mat *
		view_mat *
		vec4(f_pos, 1);
	gl_Position.z = 1.0 / (1.0 - gl_Position.z * 0.001 - 100.0);
}
