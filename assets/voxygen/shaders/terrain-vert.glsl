#version 330 core

#include <globals.glsl>
#include <srgb.glsl>

in uint v_pos_norm;
in uint v_col_light;

layout (std140)
uniform u_locals {
	vec3 model_offs;
	float load_time;
};

out vec3 f_pos;
out vec3 f_norm;
out vec3 f_col;
out float f_light;

void main() {
	f_pos = vec3((uvec3(v_pos_norm) >> uvec3(0, 8, 16)) & uvec3(0xFFu, 0xFFu, 0x1FFFu)) + model_offs;

	f_pos.z *= min(1.0001 - 0.02 / pow(tick.x - load_time, 10.0), 1.0);
	f_pos.z -= 25.0 * pow(distance(focus_pos.xy, f_pos.xy) / view_distance.x, 20.0);

	f_col = vec3((uvec3(v_col_light) >> uvec3(8, 16, 24)) & uvec3(0xFFu)) / 255.0;

	f_light = float(v_col_light & 0xFFu) / 255.0;

	// First 3 normals are negative, next 3 are positive
	vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(0,-1,0), vec3(0,0,-1), vec3(1,0,0), vec3(0,1,0), vec3(0,0,1));

	// TODO: last 3 bits in v_pos_norm should be a number between 0 and 5, rather than 0-2 and a direction.
	uint norm_axis = (v_pos_norm >> 30) & 0x3u;
	// Increase array access by 3 to access positive values
	uint norm_dir = ((v_pos_norm >> 29) & 0x1u) * 3u;
	// Use an array to avoid conditional branching
	f_norm = normals[norm_axis + norm_dir];

	gl_Position =
		all_mat *
		vec4(f_pos, 1);
	gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
