#version 330 core

#include <globals.glsl>
#include <srgb.glsl>

in vec3 v_pos;
in uint v_col;
in uint v_norm_ao;
in vec4 inst_mat0;
in vec4 inst_mat1;
in vec4 inst_mat2;
in vec4 inst_mat3;
in float inst_time;
in float inst_entropy;
in int inst_mode;

out vec3 f_pos;
flat out vec3 f_norm;
out vec3 f_col;
out float f_ao;
out float f_light;

const float SCALE = 1.0 / 11.0;

float PHI = 1.61803398874989484820459;  // Φ = Golden Ratio   

float gold_noise(in vec2 xy, in float seed){
       return fract(tan(distance(xy * PHI, xy) * seed) * xy.x);
}

// Modes
const int SMOKE = 0;
const int FIRE = 1;
const int FLAMETHROWER = 2;

void main() {
	mat4 inst_mat;
	inst_mat[0] = inst_mat0;
	inst_mat[1] = inst_mat1;
	inst_mat[2] = inst_mat2;
	inst_mat[3] = inst_mat3;

	float rand1 = gold_noise(vec2(0.0, 0.0), inst_entropy);
	float rand2 = gold_noise(vec2(1.0, 1.0), inst_entropy);
	float rand3 = gold_noise(vec2(2.0, 2.0), inst_entropy);
	float rand4 = gold_noise(vec2(3.0, 3.0), inst_entropy);
	float rand5 = gold_noise(vec2(4.0, 4.0), inst_entropy);
	float rand6 = gold_noise(vec2(5.0, 5.0), inst_entropy);

	vec3 inst_vel = vec3(0.0, 0.0, 0.0);
	vec3 inst_pos = vec3(0.0, 0.0, 0.0);
	vec3 inst_col = vec3(1.0, 1.0, 1.0);

	if (inst_mode == SMOKE) {
		inst_col = vec3(1.0, 1.0, 1.0);
		inst_vel = vec3(rand1 * 0.2 - 0.1, rand2 * 0.2 - 0.1, 1.0 + rand3);
		inst_pos = vec3(rand4 * 5.0 - 2.5, rand5 * 5.0 - 2.5, 0.0);
	} else if (inst_mode == FIRE) {
		inst_col = vec3(1.0, 1.0 * inst_entropy, 0.0);
		inst_vel = vec3(rand1 * 0.2 - 0.1, rand2 * 0.2 - 0.1, 4.0 + rand3);
		inst_pos = vec3(rand4 * 5.0 - 2.5, rand5 * 5.0 - 2.5, 0.0);
	} else if (inst_mode == FLAMETHROWER) {
		// TODO: velocity based on attack range, angle and parent orientation.
		inst_col = vec3(1.0, 1.0 * inst_entropy, 0.0);
		inst_vel = vec3(rand1 * 0.1, rand2 * 0.1, 3.0 + rand3);
		inst_pos = vec3(rand4 * 5.0 - 2.5, rand5 * 5.0 - 2.5, 0.0);
	} else {
		inst_col = vec3(rand1, rand2, rand3);
		inst_vel = vec3(rand4, rand5, rand6);
		inst_pos = vec3(rand1, rand2, rand3);
	}

	f_pos = (inst_mat * vec4((v_pos + inst_pos) * SCALE, 1)).xyz;

	f_pos += inst_vel * (tick.x - inst_time);

	// First 3 normals are negative, next 3 are positive
	vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));
	f_norm = (inst_mat * vec4(normals[(v_norm_ao >> 0) & 0x7u], 0)).xyz;

	vec3 col = vec3((uvec3(v_col) >> uvec3(0, 8, 16)) & uvec3(0xFFu)) / 255.0;
	f_col = srgb_to_linear(col) * srgb_to_linear(inst_col);
	f_ao = float((v_norm_ao >> 3) & 0x3u) / 4.0;

	f_light = 1.0;

	gl_Position =
		all_mat *
		vec4(f_pos, 1);
	gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
