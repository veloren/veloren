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
#include <lod.glsl>

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
out float f_reflect;

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
const int FIREWORK_WHITE = 8;
const int FIREWORK_YELLOW = 9;
const int LEAF = 10;
const int FIREFLY = 11;
const int BEE = 12;
const int GROUND_SHOCKWAVE = 13;
const int HEALING_BEAM = 14;
const int ENERGY_NATURE = 15;
const int FLAMETHROWER = 16;
const int FIRE_SHOCKWAVE = 17;
const int FIRE_BOWL = 18;
const int SNOW = 19;
const int EXPLOSION = 20;
const int ICE = 21;
const int LIFESTEAL_BEAM = 22;

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

float percent() {
    return lifetime / inst_lifespan;
}

float slow_end(float factor) {
    return (1 + factor) * percent() / (percent() + factor);
}

float slow_start(float factor) {
    return 1-(1 + factor) * (1-percent()) / ((1-percent()) + factor);
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

vec3 spiral_motion(vec3 line, float radius, float time_function, float frequency, float offset) {
    vec3 axis2 = perp_axis1(line);
    vec3 axis3 = perp_axis2(line, axis2);

    return line * time_function + vec3(
        radius * cos(frequency * time_function - offset) * axis2.x + radius * sin(frequency * time_function - offset) * axis3.x,
        radius * cos(frequency * time_function - offset) * axis2.y + radius * sin(frequency * time_function - offset) * axis3.y,
        radius * cos(frequency * time_function - offset) * axis2.z + radius * sin(frequency * time_function - offset) * axis3.z);
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

    vec3 start_pos = inst_pos - focus_off.xyz;

    Attr attr;
    f_reflect = 1.0;

    if (inst_mode == SMOKE) {
        attr = Attr(
            linear_motion(
                vec3(0),
                vec3(rand2 * 0.02, rand3 * 0.02, 1.0 + rand4 * 0.1)
            ),
            vec3(linear_scale(0.5)),
            vec4(vec3(0.8, 0.8, 1) * 0.5, start_end(1.0, 0.0)),
            spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 0.5)
        );
    } else if (inst_mode == FIRE) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            linear_motion(
                vec3(0.0),
                vec3(rand2 * 0.1, rand3 * 0.1, 2.0 + rand4 * 1.0)
            ),
            vec3(1.0),
            vec4(2, 1.5 + rand5 * 0.5, 0, start_end(1.0, 0.0)),
            spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
        );
        } else if (inst_mode == FIRE_BOWL) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            linear_motion(
                vec3(normalize(vec2(rand0, rand1)) * 0.1, 0.6),
                vec3(rand2 * 0.2, rand3 * 0.5, 0.8 + rand4 * 0.5)
            ),
            vec3(0.2), // Size
            vec4(2, 1.5 + rand5 * 0.5, 0, start_end(1.0, 0.0)), // Colour
            spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
        );
    } else if (inst_mode == GUN_POWDER_SPARK) {
        attr = Attr(
            linear_motion(
                normalize(vec3(rand0, rand1, rand3)) * 0.3,
                normalize(vec3(rand4, rand5, rand6)) * 2.0 + grav_vel(earth_gravity)
            ),
            vec3(1.0),
            vec4(3.5, 3 + rand7, 0, 1),
            spin_in_axis(vec3(1,0,0),0)
        );
    } else if (inst_mode == SHRAPNEL) {
        attr = Attr(
            linear_motion(
                vec3(0),
                normalize(vec3(rand4, rand5, rand6)) * 30.0 + grav_vel(earth_gravity)
            ),
            vec3(2.0 + rand0),
            vec4(vec3(0.6 + rand7 * 0.4), 1),
            spin_in_axis(vec3(1,0,0),0)
        );
    } else if (inst_mode == FIREWORK_BLUE) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            linear_motion(
                vec3(0),
                normalize(vec3(rand1, rand2, rand3)) * 40.0 + grav_vel(earth_gravity)
            ),
            vec3(3.0 + rand0),
            vec4(vec3(0, 0, 2), 1),
            identity()
        );
    } else if (inst_mode == FIREWORK_GREEN) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            linear_motion(
                vec3(0),
                normalize(vec3(rand1, rand2, rand3)) * 40.0 + grav_vel(earth_gravity)
            ),
            vec3(3.0 + rand0),
            vec4(vec3(0, 2, 0), 1),
            identity()
        );
    } else if (inst_mode == FIREWORK_PURPLE) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            linear_motion(
                vec3(0),
                normalize(vec3(rand1, rand2, rand3)) * 40.0 + grav_vel(earth_gravity)
            ),
            vec3(3.0 + rand0),
            vec4(vec3(2, 0, 2), 1),
            identity()
        );
    } else if (inst_mode == FIREWORK_RED) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            linear_motion(
                vec3(0),
                normalize(vec3(rand1, rand2, rand3)) * 40.0 + grav_vel(earth_gravity)
            ),
            vec3(3.0 + rand0),
            vec4(vec3(2, 0, 0), 1),
            identity()
        );
        } else if (inst_mode == FIREWORK_WHITE) {
            f_reflect = 0.0; // Fire doesn't reflect light, it emits it
            attr = Attr(
                linear_motion(
                    vec3(0),
                    normalize(vec3(rand1, rand2, rand3)) * 40.0 + grav_vel(earth_gravity)
                ),
                vec3(3.0 + rand0),
                vec4(vec3(2, 2, 2), 1),
                identity()
            );
    } else if (inst_mode == FIREWORK_YELLOW) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            linear_motion(
                vec3(0),
                normalize(vec3(rand1, rand2, rand3)) * 40.0 + grav_vel(earth_gravity)
            ),
            vec3(3.0 + rand0),
            vec4(vec3(2, 2, 0), 1),
            identity()
        );
    } else if (inst_mode == LEAF) {
        attr = Attr(
            linear_motion(
                vec3(0),
                vec3(0, 0, -2)
            ) + vec3(sin(lifetime), sin(lifetime + 0.7), sin(lifetime * 0.5)) * 2.0,
            vec3(4),
            vec4(vec3(0.2 + rand7 * 0.2, 0.2 + (0.25 + rand6 * 0.5) * 0.3, 0) * (0.75 + rand1 * 0.5), 1),
            spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
        );
    } else if (inst_mode == SNOW) {
        float height = mix(-4, 60, pow(start_end(1, 0), 3));
        float wind_speed = (inst_pos.z - 250) * 0.025;
        vec3 offset = linear_motion(vec3(0), vec3(1, 1, 0) * wind_speed);
        float end_alt = alt_at(start_pos.xy + offset.xy);
        attr = Attr(
            offset + vec3(0, 0, end_alt - start_pos.z + height) + vec3(sin(lifetime), sin(lifetime + 0.7), sin(lifetime * 0.5)) * 3,
            vec3(mix(4, 0, pow(start_end(1, 0), 4))),
            vec4(1),
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
        f_reflect = 0.0;
        attr = Attr(
            spiral_motion(inst_dir, 0.3 * (floor(2 * rand0 + 0.5) - 0.5) * min(linear_scale(10), 1), lifetime / inst_lifespan, 10.0, inst_time),
            vec3((1.7 - 0.7 * abs(floor(2 * rand0 - 0.5) + 0.5)) * (1.5 + 0.5 * sin(tick.x * 10 - lifetime * 4))),
            vec4(vec3(0.4, 1.6 + 0.3 * sin(tick.x * 10 - lifetime * 3 + 4), 1.0 + 0.15 * sin(tick.x * 5 - lifetime * 5)), 1 /*0.3*/),
            spin_in_axis(inst_dir, tick.z)
        );
    } else if (inst_mode == LIFESTEAL_BEAM) {
        f_reflect = 0.0;
        float green_col = 0.2 + 1.4 * sin(tick.x * 5 + lifetime * 5);
        float purple_col = 1.2 + 0.1 * sin(tick.x * 3 - lifetime * 3) - max(green_col, 1) + 1;
        attr = Attr(
            spiral_motion(inst_dir, 0.3 * (floor(2 * rand0 + 0.5) - 0.5) * min(linear_scale(10), 1), lifetime / inst_lifespan, 10.0, inst_time),
            vec3((1.7 - 0.7 * abs(floor(2 * rand0 - 0.5) + 0.5)) * (1.5 + 0.5 * sin(tick.x * 10 - lifetime * 4))),
            vec4(vec3(purple_col, green_col, 0.75 * purple_col), 1),
            spin_in_axis(inst_dir, tick.z)
        );
    } else if (inst_mode == ENERGY_NATURE) {
        f_reflect = 0.0;
        float spiral_radius = start_end(1 - pow(abs(rand5), 5), 1) * length(inst_dir);
        attr = Attr(
            spiral_motion(vec3(0, 0, rand3 + 1), spiral_radius, lifetime, abs(rand0), rand1 * 2 * PI) + vec3(0, 0, rand2),
            vec3(6 * abs(rand4) * (1 - slow_start(2)) * pow(spiral_radius / length(inst_dir), 0.5)),
            vec4(vec3(0, 1.7, 1.3), 1),
            spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
        );
    } else if (inst_mode == FLAMETHROWER) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (lifetime * 5 + 0.25),
            vec3((2.5 * (1 - slow_start(0.3)))),
            vec4(3, 1.6 + rand5 * 0.3 - 0.4 * percent(), 0.2, 1),
            spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
        );
    } else if (inst_mode == EXPLOSION) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            inst_dir * ((rand0+1.0)/2 + 0.4) * slow_end(2.0) + 0.3 * grav_vel(earth_gravity),
            vec3((3 * (1 - slow_start(0.1)))),
            vec4(3, 1.6 + rand5 * 0.3 - 0.4 * percent(), 0.2, 1),
            spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
        );
    } else if (inst_mode == ICE) {
        f_reflect = 0.0; // Ice doesn't reflect to look like magic
        attr = Attr(
            inst_dir * ((rand0+1.0)/2 + 0.4) * slow_end(2.0) + 0.3 * grav_vel(earth_gravity),
            vec3((3 * (1 - slow_start(0.1)))),
            vec4(0.2, 1.6 + rand5 * 0.3 - 0.4 * percent(), 3, 1),
            spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
        );
    } else if (inst_mode == FIRE_SHOCKWAVE) {
        f_reflect = 0.0; // Fire doesn't reflect light, it emits it
        attr = Attr(
            vec3(rand0, rand1, lifetime * 10 + rand2),
            vec3((5 * (1 - slow_start(0.5)))),
            vec4(3, 1.6 + rand5 * 0.3 - 0.4 * percent(), 0.2, 1),
            spin_in_axis(vec3(rand3, rand4, rand5), rand6)
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

    // Temporary: use shrinking particles as a substitute for fading ones
    attr.scale *= pow(attr.col.a, 0.25);

    f_pos = start_pos + (v_pos * attr.scale * SCALE * mat3(attr.rot) + attr.offs);

    // First 3 normals are negative, next 3 are positive
    // TODO: Make particle normals match orientation
    vec4 normals[6] = vec4[](vec4(-1,0,0,0), vec4(1,0,0,0), vec4(0,-1,0,0), vec4(0,1,0,0), vec4(0,0,-1,0), vec4(0,0,1,0));
    f_norm =
        // inst_pos *
        ((normals[(v_norm_ao >> 0) & 0x7u]) * attr.rot).xyz;

    //vec3 col = vec3((uvec3(v_col) >> uvec3(0, 8, 16)) & uvec3(0xFFu)) / 255.0;
    f_col = vec4(attr.col.rgb, attr.col.a);

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
}
