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
out vec3 f_norm;
out float f_light;

void main() {
	f_pos = lod_pos(v_pos);

	f_norm = lod_norm(f_pos.xy);

	//f_pos.z -= 1.0 / pow(distance(focus_pos.xy, f_pos.xy) / (view_distance.x * 0.95), 20.0);

	//f_pos.z -= 100.0 * pow(1.0 + 0.01 / view_distance.x, -pow(distance(focus_pos.xy, f_pos.xy), 2.0));
	f_pos.z -= max(view_distance.x - distance(focus_pos.xy, f_pos.xy), 0.0);

	f_light = 1.0;

	gl_Position =
		proj_mat *
		view_mat *
		vec4(f_pos, 1);
	gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
