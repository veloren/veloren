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
in vec3 inst_dir;
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
const int FIREFLY = 10;
const int BEE = 11;
const int GROUND_SHOCKWAVE = 12;
const int HEALING_BEAM = 13;
const int ENERGY_NATURE = 14;
const int FLAMETHROWER = 15;

// meters per second squared (acceleration)
const float earth_gravity = 9.807;

struct Attr {
	vec3 offs;
	vec3 scale;
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

mat4 identity() {
	return mat4(
		1, 0, 0, 0,
		0, 1, 0, 0,
		0, 0, 1, 0,
		0, 0, 0, 1
	);
}

vec3 perp_axis1(vec3 axis) {
	return normalize(vec3(axis.y + axis.z, -axis.x + axis.z, -axis.x - axis.y));
}

vec3 perp_axis2(vec3 axis1, vec3 axis2) {
	return normalize(vec3(axis1.y * axis2.z - axis1.z * axis2.y, axis1.z * axis2.x - axis1.x * axis2.z, axis1.x * axis2.y - axis1.y * axis2.x));
}

vec3 spiral_motion(vec3 line, float radius, float time_function) {
	vec3 axis2 = perp_axis1(line);
	vec3 axis3 = perp_axis2(line, axis2);

	return line * time_function + vec3(
		radius * cos(10 * time_function - inst_time) * axis2.x + radius * sin(10 * time_function - inst_time) * axis3.x,
		radius * cos(10 * time_function - inst_time) * axis2.y + radius * sin(10 * time_function - inst_time) * axis3.y,
		radius * cos(10 * time_function - inst_time) * axis2.z + radius * sin(10 * time_function - inst_time) * axis3.z);
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
				vec3(0),
				vec3(rand2 * 0.02, rand3 * 0.02, 1.0 + rand4 * 0.1)
			),
			vec3(linear_scale(0.5)),
			vec4(1, 1, 1, start_end(1.0, 0.0)),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 0.5)
		);
	} else if (inst_mode == FIRE) {
		attr = Attr(
			linear_motion(
				vec3(rand0 * 0.25, rand1 * 0.25, 0.3),
				vec3(rand2 * 0.1, rand3 * 0.1, 2.0 + rand4 * 1.0)
			),
			vec3(1.0),
			vec4(2, 0.8 + rand5 * 0.3, 0, 1),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
		);
	} else if (inst_mode == GUN_POWDER_SPARK) {
		attr = Attr(
			linear_motion(
				vec3(rand0, rand1, rand3) * 0.3,
				vec3(rand4, rand5, rand6) * 2.0 + grav_vel(earth_gravity)
			),
			vec3(1.0),
			vec4(3.5, 3 + rand7, 0, 1),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == SHRAPNEL) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(rand4, rand5, rand6) * 40.0 + grav_vel(earth_gravity)
			),
			vec3(3.0 + rand0),
			vec4(vec3(0.6 + rand7 * 0.4), 1),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == FIREWORK_BLUE) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(rand1, rand2, rand3) * 40.0 + grav_vel(earth_gravity)
			),
			vec3(3.0 + rand0),
			vec4(0.15, 0.4, 1, 1),
			identity()
		);
	} else if (inst_mode == FIREWORK_GREEN) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(rand1, rand2, rand3) * 40.0 + grav_vel(earth_gravity)
			),
			vec3(3.0 + rand0),
			vec4(0, 1, 0, 1),
			identity()
		);
	} else if (inst_mode == FIREWORK_PURPLE) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(rand1, rand2, rand3) * 40.0 + grav_vel(earth_gravity)
			),
			vec3(3.0 + rand0),
			vec4(0.7, 0.0, 1.0, 1.0),
			identity()
		);
	} else if (inst_mode == FIREWORK_RED) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(rand1, rand2, rand3) * 40.0 + grav_vel(earth_gravity)
			),
			vec3(3.0 + rand0),
			vec4(1, 0, 0, 1),
			identity()
		);
	} else if (inst_mode == FIREWORK_YELLOW) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(rand1, rand2, rand3) * 40.0 + grav_vel(earth_gravity)
			),
			vec3(3.0 + rand0),
			vec4(1, 1, 0, 1),
			identity()
		);
	} else if (inst_mode == LEAF) {
		attr = Attr(
			linear_motion(
				vec3(0),
				vec3(0, 0, -2)
			) + vec3(sin(lifetime), sin(lifetime + 0.7), sin(lifetime * 0.5)) * 2.0,
			vec3(4),
			vec4(vec3(0.2 + rand7 * 0.2, 0.2 + (0.5 + rand6 * 0.5) * 0.6, 0), 1),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
		);
	} else if (inst_mode == FIREFLY) {
		float raise = pow(sin(3.1416 * lifetime / inst_lifespan), 0.2);
		attr = Attr(
			vec3(0, 0, raise * 5.0) + vec3(
				sin(lifetime * 1.0 + rand0) + sin(lifetime * 7.0 + rand3) * 0.3,
				sin(lifetime * 3.0 + rand1) + sin(lifetime * 8.0 + rand4) * 0.3,
				sin(lifetime * 2.0 + rand2) + sin(lifetime * 9.0 + rand5) * 0.3
			),
			vec3(raise),
			vec4(vec3(5, 5, 1.1), 1),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
		);
	} else if (inst_mode == BEE) {
		float lower = pow(sin(3.1416 * lifetime / inst_lifespan), 0.2);
		attr = Attr(
			vec3(0, 0, lower * -0.5) + vec3(
				sin(lifetime * 2.0 + rand0) + sin(lifetime * 9.0 + rand3) * 0.3,
				sin(lifetime * 3.0 + rand1) + sin(lifetime * 10.0 + rand4) * 0.3,
				sin(lifetime * 4.0 + rand2) + sin(lifetime * 11.0 + rand5) * 0.3
			) * 0.5,
			vec3(lower),
			vec4(vec3(1, 0.7, 0), 1),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
		);
	} else if (inst_mode == GROUND_SHOCKWAVE) {
		attr = Attr(
			vec3(0.0),
			vec3(11.0, 11.0, (33.0 * rand0 * sin(2.0 * lifetime * 3.14 * 2.0))) / 3,
			vec4(vec3(0.32 + (rand0 * 0.04), 0.22 + (rand1 * 0.03), 0.05 + (rand2 * 0.01)), 1),
			spin_in_axis(vec3(1,0,0),0)
		);
	} else if (inst_mode == HEALING_BEAM) {
		attr = Attr(
			spiral_motion(inst_dir, 0.3 * (floor(2 * rand0 + 0.5) - 0.5) * min(linear_scale(10), 1), lifetime / inst_lifespan),
			vec3((1.7 - 0.7 * abs(floor(2 * rand0 - 0.5) + 0.5)) * (1.5 + 0.5 * sin(tick.x * 10 - lifetime * 4))),
			vec4(vec3(0.3, 0.7 + 0.4 * sin(tick.x * 8 - lifetime * 3), 0.3 + 0.1 * sin (tick.x * 2)), 0.3),
			spin_in_axis(inst_dir, tick.z)
		);
	} else if (inst_mode == ENERGY_NATURE) {
		attr = Attr(
			linear_motion(
				vec3(rand0 * 1, rand1 * 1, rand2 * 1),
				vec3(rand3 * 2, rand4 * 2, rand5 * 2)
			),
			vec3(0.8),
			vec4(vec3(0, 1, 0), 1),
			spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
		);
	} else if (inst_mode == FLAMETHROWER) {
		attr = Attr(
			(inst_dir * lifetime / inst_lifespan) + vec3(rand0, rand1, rand2) * 0.3,
			vec3(0.6 + rand3 * 0.5 + lifetime / inst_lifespan * 5),
			vec4(1, 0.6 + rand5 * 0.3 - 0.6 * lifetime / inst_lifespan, 0, 0.8 - 0.6 * lifetime / inst_lifespan),
			spin_in_axis(vec3(rand6, rand7, rand8), lifetime / inst_lifespan * 10 + 3 * rand9)
		);
	} else {
		attr = Attr(
			linear_motion(
				vec3(rand0 * 0.25, rand1 * 0.25, 1.7 + rand5),
				vec3(rand2 * 0.1, rand3 * 0.1, 1.0 + rand4 * 0.5)
			),
			vec3(exp_scale(-0.2)),
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
