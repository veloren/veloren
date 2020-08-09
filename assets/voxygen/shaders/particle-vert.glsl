#version 330 core

#include <globals.glsl>
#include <srgb.glsl>
#include <random.glsl>

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

// Modes
const int SMOKE = 0;
const int FIRE = 1;
const int GUN_POWDER_SPARK = 2;
const int SHRAPNEL = 3;

// meters per second squared (acceleration)
const float earth_gravity = 9.807;

struct Attr {
	vec3 offs;
	float scale;
	vec3 col;
};

float lifetime = tick.x - inst_time;

vec3 linear_motion(vec3 init_offs, vec3 vel) {
	return init_offs + vel * lifetime;
}

vec3 grav_vel(float grav) {
	return vec3(0, 0, -grav * lifetime);
}

float exp_scale(float factor) {
	return 1 / (1 - lifetime * factor);
}

void main() {
	float rand0 = hash(vec4(inst_entropy + 0));
	float rand1 = hash(vec4(inst_entropy + 1));
	float rand2 = hash(vec4(inst_entropy + 2));
	float rand3 = hash(vec4(inst_entropy + 3));
	float rand4 = hash(vec4(inst_entropy + 4));
	float rand5 = hash(vec4(inst_entropy + 5));
	float rand6 = hash(vec4(inst_entropy + 6));
	float rand7 = hash(vec4(inst_entropy + 7));

	Attr attr;

	if (inst_mode == SMOKE) {
		attr = Attr(
			linear_motion(
				vec3(rand0 * 0.25, rand1 * 0.25, 1.7 + rand5),
				vec3(rand2 * 0.2, rand3 * 0.2, 1.0 + rand4 * 0.5)// + vec3(sin(lifetime), sin(lifetime + 1.5), sin(lifetime * 4) * 0.25)
			),
			exp_scale(-0.2),
			vec3(1)
		);
	} else if (inst_mode == FIRE) {
		attr = Attr(
			linear_motion(
				vec3(rand0 * 0.25, rand1 * 0.25, 0.3),
				vec3(rand2 * 0.1, rand3 * 0.1, 2.0 + rand4 * 1.0)
			),
			1.0,
			vec3(2, rand5 + 2, 0)
		);
	} else if (inst_mode == GUN_POWDER_SPARK) {
		attr = Attr(
			linear_motion(
				vec3(rand0, rand1, rand3) * 0.3,
				vec3(rand4, rand5, rand6) * 2.0 + grav_vel(earth_gravity)
			),
			1.0,
			vec3(3.5, 3 + rand7, 0)
		);
	} else if (inst_mode == SHRAPNEL) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(rand4, rand5, rand6) * 40.0 + grav_vel(earth_gravity)
			),
			3.0 + rand0,
			vec3(0.6 + rand7 * 0.4)
		);
	} else {
		attr = Attr(
			linear_motion(
				vec3(rand0 * 0.25, rand1 * 0.25, 1.7 + rand5),
				vec3(rand2 * 0.1, rand3 * 0.1, 1.0 + rand4 * 0.5)
			),
			exp_scale(-0.2),
			vec3(1)
		);
	}

	f_pos = inst_pos + (v_pos * attr.scale * SCALE + attr.offs);

	// First 3 normals are negative, next 3 are positive
	vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));
	f_norm = 
		// inst_pos *
		normals[(v_norm_ao >> 0) & 0x7u];

	//vec3 col = vec3((uvec3(v_col) >> uvec3(0, 8, 16)) & uvec3(0xFFu)) / 255.0;
	f_col = 
		//srgb_to_linear(col) * 
		srgb_to_linear(attr.col);
	f_ao = float((v_norm_ao >> 3) & 0x3u) / 4.0;

	f_light = 1.0;

	gl_Position =
		all_mat *
		vec4(f_pos, 1);
	gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
