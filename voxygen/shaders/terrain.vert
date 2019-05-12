#version 330 core

#include <globals.glsl>

in vec3 v_pos;
in vec3 v_norm;
in vec3 v_col;

layout (std140)
uniform u_locals {
	vec3 model_offs;
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
