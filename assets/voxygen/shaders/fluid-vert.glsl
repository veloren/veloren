#version 330 core

#include <globals.glsl>

in uint v_pos_norm;
in uint v_col_light;

layout (std140)
uniform u_locals {
    vec3 model_offs;
};

out vec3 f_pos;
flat out vec3 f_norm;
out vec3 f_col;
out float f_light;
out float f_opac;

// First 3 normals are negative, next 3 are positive
vec3 normals[6] = vec3[]( vec3(-1,0,0), vec3(0,-1,0), vec3(0,0,-1), vec3(1,0,0), vec3(0,1,0), vec3(0,0,1) );

void main() {
    f_pos = vec3(
    	float((v_pos_norm >>  0) & 0x00FFu),
    	float((v_pos_norm >>  8) & 0x00FFu),
    	float((v_pos_norm >> 16) & 0x1FFFu)
    ) + model_offs;

    // TODO: last 3 bits in v_pos_norm should be a number between 0 and 5, rather than 0-2 and a direction.
    uint norm_axis = (v_pos_norm >> 30) & 0x3u;

    // Increase array access by 3 to access positive values
    uint norm_dir = ((v_pos_norm >> 29) & 0x1u) * 3u;

    // Use an array to avoid conditional branching
    f_norm = normals[norm_axis + norm_dir];

    f_col = vec3(
    	float((v_col_light >>  8) & 0xFFu),
    	float((v_col_light >> 16) & 0xFFu),
    	float((v_col_light >> 24) & 0xFFu)
    ) / 200.0;

    f_light = float(v_col_light & 0xFFu) / 255.0;

	f_opac = 0.3;

    gl_Position =
		proj_mat *
		view_mat *
		vec4(f_pos, 1);
}
