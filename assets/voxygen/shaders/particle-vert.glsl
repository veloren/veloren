#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <srgb.glsl>
#include <random.glsl>

in vec3 v_pos;
// in uint v_col;
in uint v_norm_ao;
in vec3 inst_pos;
in float inst_time;
in float inst_lifespan;
in float inst_entropy;
in int inst_mode;

out vec3 f_pos;
flat out vec3 f_norm;
out vec4 f_col;
out float f_ao;
out float f_light;

const float SCALE = 1.0 / 11.0;

// Modes
const int SMOKE = 0;
const int FIRE = 1;
const int GUN_POWDER_SPARK = 2;
const int SHRAPNEL = 3;

const int FIREWORK_BLUE = 4;
const int FIREWORK_GREEN = 5;
const int FIREWORK_PURPLE = 6;
const int FIREWORK_RED = 7;
const int FIREWORK_YELLOW = 8;
const int LEAF = 9;

// meters per second squared (acceleration)
const float earth_gravity = 9.807;

struct Attr {
	vec3 offs;
	float scale;
	vec4 col;
	mat4 rot;
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

float linear_scale(float factor) {
	return lifetime * factor;
}

float start_end(float from, float to) {
	return mix(from, to, lifetime / inst_lifespan);
}

mat4 spin_in_axis(vec3 axis, float angle)
{
	axis = normalize(axis);
	float s = sin(angle);
	float c = cos(angle);
	float oc = 1.0 - c;

	return mat4(oc * axis.x * axis.x + c,  oc * axis.x * axis.y - axis.z * s, oc * axis.z * axis.x + axis.y * s, 0,
		oc * axis.x * axis.y + axis.z * s, oc * axis.y * axis.y + c,          oc * axis.y * axis.z - axis.x * s, 0,
		oc * axis.z * axis.x - axis.y * s, oc * axis.y * axis.z + axis.x * s, oc * axis.z * axis.z + c,          0,
		0,                                 0,                                 0,                                 1);
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
	float rand8 = hash(vec4(inst_entropy + 8));
	float rand9 = hash(vec4(inst_entropy + 9));

	Attr attr;

	if (inst_mode == SMOKE) {
		attr = Attr(
			linear_motion(
				vec3(0.0, 0.0, 0.0),
				vec3(rand2 * 0.1, rand3 * 0.1, 1.0 + rand4 * 0.1)
			),
			linear_scale(0.2),
			vec4(1, 1, 1, start_end(0.1, 0.0)),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 0.5)
		);
	} else if (inst_mode == FIRE) {
		attr = Attr(
			linear_motion(
				vec3(rand0 * 0.3, rand1 * 0.3, 0.2),
				vec3(rand1 * 0.1, rand3 * 0.1, 3.0 + rand4 * 1.2)
			),
			1.3,
			vec4(2, 0.8 + rand5 * 0.3, 0, 1),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
		);
	} else if (inst_mode == GUN_POWDER_SPARK) {
		attr = Attr(
			linear_motion(
				vec3(rand0, rand1, rand3) * 0.3,
				vec3(rand4, rand5, rand6) * 2.0 + grav_vel(earth_gravity)
			),
			1.0,
			vec4(3.5, 3 + rand7, 0, 1),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == SHRAPNEL) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(rand4, rand5, rand6) * 40.0 + grav_vel(earth_gravity)
			),
			3.0 + rand0,
			vec4(vec3(0.6 + rand7 * 0.4), 1),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == FIREWORK_BLUE) {
		attr = Attr(
			linear_motion(
				vec3(0.0, 1.0, 1.0),
				vec3(rand4, rand5, rand6) * 40.0 + grav_vel(earth_gravity)
			),
			3.0 + rand0,
			vec4(vec3(0.6 + rand7 * 0.4), 0.3),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == FIREWORK_GREEN) {
		attr = Attr(
			linear_motion(
				vec3(0.0, 1.0, 0.0),
				vec3(rand4, rand5, rand6) * 40.0 + grav_vel(earth_gravity)
			),
			3.0 + rand0,
			vec4(vec3(0.6 + rand7 * 0.4), 0.3),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == FIREWORK_PURPLE) {
		attr = Attr(
			linear_motion(
				vec3(1.0, 0.0, 1.0),
				vec3(rand4, rand5, rand6) * 40.0 + grav_vel(earth_gravity)
			),
			3.0 + rand0,
			vec4(vec3(0.6 + rand7 * 0.4), 0.3),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == FIREWORK_RED) {
		attr = Attr(
			linear_motion(
				vec3(1.0, 0.0, 0.0),
				vec3(rand4, rand5, rand6) * 40.0 + grav_vel(earth_gravity)
			),
			3.0 + rand0,
			vec4(vec3(0.6 + rand7 * 0.4), 0.3),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == FIREWORK_YELLOW) {
		attr = Attr(
			linear_motion(
				vec3(1.0, 1.0, 0.0),
				vec3(rand4, rand5, rand6) * 40.0 + grav_vel(earth_gravity)
			),
			3.0 + rand0,
			vec4(vec3(0.6 + rand7 * 0.4), 0.3),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == LEAF) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(0, 0, -2)
			) + vec3(sin(lifetime), sin(lifetime + 0.7), sin(lifetime * 0.5)) * 2.0,
			4,
			vec4(vec3(0.2 + rand7 * 0.2, 0.2 + (0.5 + rand6 * 0.5) * 0.6, 0), 1),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
		);
	} else {
		attr = Attr(
			linear_motion(
				vec3(rand0 * 0.25, rand1 * 0.25, 1.7 + rand5),
				vec3(rand2 * 0.1, rand3 * 0.1, 1.0 + rand4 * 0.5)
			),
			exp_scale(-0.2),
			vec4(1),
			spin_in_axis(vec3(1,0,0),0)
		);
	}

	f_pos = (inst_pos - focus_off.xyz) + (v_pos * attr.scale * SCALE * mat3(attr.rot) + attr.offs);

	// First 3 normals are negative, next 3 are positive
	// TODO: Make particle normals match orientation
	vec4 normals[6] = vec4[](vec4(-1,0,0,0), vec4(1,0,0,0), vec4(0,-1,0,0), vec4(0,1,0,0), vec4(0,0,-1,0), vec4(0,0,1,0));
	f_norm =
		// inst_pos *
		((normals[(v_norm_ao >> 0) & 0x7u]) * attr.rot).xyz;

	//vec3 col = vec3((uvec3(v_col) >> uvec3(0, 8, 16)) & uvec3(0xFFu)) / 255.0;
	f_col = vec4(srgb_to_linear(attr.col.rgb), attr.col.a);

	gl_Position =
		all_mat *
		vec4(f_pos, 1);
}
