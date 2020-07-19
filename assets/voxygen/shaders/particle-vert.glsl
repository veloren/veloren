#version 330 core

#include <globals.glsl>
#include <srgb.glsl>

in vec3 v_pos;
in uint v_col;
in uint v_norm_ao;
in vec3 inst_pos;
in float inst_time;
in float inst_entropy;
in int inst_mode;

out vec3 f_pos;
flat out vec3 f_norm;
out vec3 f_col;
out float f_ao;
out float f_light;

const float SCALE = 1.0 / 11.0;

// Φ = Golden Ratio
float PHI = 1.61803398874989484820459;

float gold_noise(in vec2 xy, in float seed){
       return fract(tan(distance(xy * PHI, xy) * seed) * xy.x);
}

// Modes
const int SMOKE = 0;
const int FIRE = 1;
const int GUN_POWDER_SPARK = 2;

// meters per second
const float earth_gravity = 9.807;

mat4 translate(vec3 vec){
    return mat4(
        vec4(1.0,   0.0,   0.0,   0.0),
        vec4(0.0,   1.0,   0.0,   0.0),
        vec4(0.0,   0.0,   1.0,   0.0),
        vec4(vec.x, vec.y, vec.z, 1.0)
    );
}

void main() {
	mat4 inst_mat = translate(inst_pos);

	float rand1 = gold_noise(vec2(0.0, 0.0), inst_entropy);
	float rand2 = gold_noise(vec2(10.0, 10.0), inst_entropy);
	float rand3 = gold_noise(vec2(20.0, 20.0), inst_entropy);
	float rand4 = gold_noise(vec2(30.0, 30.0), inst_entropy);
	float rand5 = gold_noise(vec2(40.0, 40.0), inst_entropy);
	float rand6 = gold_noise(vec2(50.0, 50.0), inst_entropy);

	vec3 inst_vel = vec3(0.0, 0.0, 0.0);
	vec3 inst_pos2 = vec3(0.0, 0.0, 0.0);
	vec3 inst_col = vec3(1.0, 1.0, 1.0);

	if (inst_mode == SMOKE) {
		inst_col = vec3(1.0, 1.0, 1.0);
		inst_vel = vec3(rand1 * 0.2 - 0.1, rand2 * 0.2 - 0.1, 1.0 + rand3);
		inst_pos2 = vec3(rand4 * 5.0 - 2.5, rand5 * 5.0 - 2.5, 0.0);
	} else if (inst_mode == FIRE) {
		inst_col = vec3(1.0, 1.0 * inst_entropy, 0.0);
		inst_vel = vec3(rand1 * 0.2 - 0.1, rand2 * 0.2 - 0.1, 4.0 + rand3);
		inst_pos2 = vec3(rand4 * 5.0 - 2.5, rand5 * 5.0 - 2.5, 0.0);
	} else if (inst_mode == GUN_POWDER_SPARK) {
		inst_col = vec3(1.0, 1.0, 0.0);
		inst_vel = vec3(rand2 * 2.0 - 1.0, rand1 * 2.0 - 1.0, 5.0 + rand3);
		inst_vel -= vec3(0.0, 0.0, earth_gravity * (tick.x - inst_time));
		inst_pos2 = vec3(0.0, 0.0, 0.0);
	} else {
		inst_col = vec3(rand1, rand2, rand3);
		inst_vel = vec3(rand4, rand5, rand6);
		inst_pos2 = vec3(rand1, rand2, rand3);
	}

	f_pos = (inst_mat * vec4((v_pos + inst_pos2) * SCALE, 1)).xyz;

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
