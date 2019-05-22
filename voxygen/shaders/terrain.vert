#version 330 core

#include <globals.glsl>

in uint v_pos;
in uint v_col_norm;

layout (std140)
uniform u_locals {
	vec3 model_offs;
};

out vec3 f_pos;
out vec3 f_norm;
out vec3 f_col;

void main() {
	f_pos = vec3(
		float((v_pos >>  0) & 0x00FFu),
		float((v_pos >>  8) & 0x00FFu),
		float((v_pos >> 16) & 0xFFFFu)
	);

	f_norm = vec3(
		float((v_col_norm >> 0) & 0x3u),
		float((v_col_norm >> 2) & 0x3u),
		float((v_col_norm >> 4) & 0x3u)
	) - 1.0;

	f_col = vec3(
		float((v_col_norm >>  8) & 0xFFu),
		float((v_col_norm >> 16) & 0xFFu),
		float((v_col_norm >> 24) & 0xFFu)
	) / 255.0;

	gl_Position =
		proj_mat *
		view_mat *
		vec4(f_pos + model_offs, 1);
}
