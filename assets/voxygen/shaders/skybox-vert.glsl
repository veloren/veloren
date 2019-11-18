#version 330 core

#include <globals.glsl>

in vec3 v_pos;

layout (std140)
uniform u_locals {
	vec4 nul;
};

out vec3 f_pos;

void main() {
	f_pos = v_pos;

	// TODO: Make this position-independent to avoid rounding error jittering
	gl_Position =
		proj_mat *
		view_mat *
		vec4(v_pos * 2000.0 + cam_pos.xyz, 1);
}
