#version 330 core

#include <globals.glsl>
#include <srgb.glsl>

in vec3 v_pos;
in vec3 v_norm;
in vec3 v_col;
in float v_ao;
in vec4 inst_mat0;
in vec4 inst_mat1;
in vec4 inst_mat2;
in vec4 inst_mat3;
in vec3 inst_col;
in float inst_wind_sway;

out vec3 f_pos;
flat out vec3 f_norm;
out vec3 f_col;
out float f_ao;
out float f_light;

const float SCALE = 1.0 / 11.0;

void main() {
	mat4 inst_mat;
	inst_mat[0] = inst_mat0;
	inst_mat[1] = inst_mat1;
	inst_mat[2] = inst_mat2;
	inst_mat[3] = inst_mat3;

	vec3 sprite_pos = (inst_mat * vec4(0, 0, 0, 1)).xyz;

	f_pos = (inst_mat * vec4(v_pos * SCALE, 1)).xyz;
	f_pos.z -= 25.0 * pow(distance(focus_pos.xy, f_pos.xy) / view_distance.x, 20.0);

	// Wind waving
	f_pos += inst_wind_sway * vec3(
		sin(tick.x * 1.5 + f_pos.y * 0.1) * sin(tick.x * 0.35),
		sin(tick.x * 1.5 + f_pos.x * 0.1) * sin(tick.x * 0.25),
		0.0
	) * pow(abs(v_pos.z) * SCALE, 1.3) * 0.2;

	f_norm = (inst_mat * vec4(v_norm, 0)).xyz;

	f_col = srgb_to_linear(v_col) * srgb_to_linear(inst_col);
	f_ao = v_ao;

	// Select glowing
	if (select_pos.w > 0 && select_pos.xyz == floor(sprite_pos)) {
		f_col *= 4.0;
	}

	f_light = 1.0;

	gl_Position =
		all_mat *
		vec4(f_pos, 1);
	gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
