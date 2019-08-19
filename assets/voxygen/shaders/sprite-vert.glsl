#version 330 core

#include <globals.glsl>
#include <srgb.glsl>

in vec3 v_pos;
in vec3 v_norm;
in vec3 v_col;
in vec3 inst_pos;
in vec3 inst_col;

out vec3 f_pos;
flat out vec3 f_norm;
out vec3 f_col;
out float f_light;

const float SCALE = 1.0 / 11.0;

void main() {
	f_pos = inst_pos + v_pos * SCALE;

	f_norm = v_norm;

	f_col = v_col * inst_col;

	f_light = 1.0;

	gl_Position =
		proj_mat *
		view_mat *
		vec4(f_pos, 1);
}
