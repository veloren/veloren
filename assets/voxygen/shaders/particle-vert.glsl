#version 420 core

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

layout(location = 0) in vec3 v_pos;
// in uint v_col;
layout(location = 1) in uint v_norm_ao;
layout(location = 2) in float inst_time;
layout(location = 3) in float inst_lifespan;
layout(location = 4) in float inst_entropy;
layout(location = 5) in int inst_mode;
layout(location = 6) in vec3 inst_dir;
layout(location = 7) in vec3 inst_pos;

layout(location = 0) out vec3 f_pos;
layout(location = 1) flat out vec3 f_norm;
layout(location = 2) out vec4 f_col;
//layout(location = x) out float f_ao;
//layout(location = x) out float f_light;
layout(location = 3) out float f_reflect;

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
const int ENERGY_HEALING = 14;
const int ENERGY_NATURE = 15;
const int FLAMETHROWER = 16;
const int FIRE_SHOCKWAVE = 17;
const int FIRE_BOWL = 18;
const int SNOW = 19;
const int EXPLOSION = 20;
const int ICE = 21;
const int LIFESTEAL_BEAM = 22;
const int CULTIST_FLAME = 23;
const int STATIC_SMOKE = 24;
const int BLOOD = 25;
const int ENRAGED = 26;
const int BIG_SHRAPNEL = 27;
const int LASER = 28;
const int BUBBLES = 29;
const int WATER = 30;
const int ICE_SPIKES = 31;
const int DRIP = 32;
const int TORNADO = 33;
const int DEATH = 34;
const int ENERGY_BUFFING = 35;
const int WEB_STRAND = 36;
const int BLACK_SMOKE = 37;
const int LIGHTNING = 38;
const int STEAM = 39;
const int BARRELORGAN = 40;
const int POTION_SICKNESS = 41;
const int GIGA_SNOW = 42;

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

vec3 quadratic_bezier_motion(vec3 start, vec3 ctrl0, vec3 end) {
    float t = lifetime;
    float u = 1 - lifetime;
    return u*u*start + t*u*ctrl0 + t*t*end;
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

// Line is the axis of the spiral, it goes from the start position to the end position
// Radius is the distance from the axis the particle is
// Time function is some value that ideally goes from 0 to 1. When it is 0, it is as
// the point (0, 0, 0), when it is 1, it is at the point provided by the coordinates of line
// Frequency increases the frequency of rotation
// Offset is an offset to the angle of the rotation
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

    switch(inst_mode) {
        case SMOKE:
            attr = Attr(
                linear_motion(
                    vec3(0),
                    vec3(rand2 * 0.02, rand3 * 0.02, 1.0 + rand4 * 0.1)
                ),
                vec3(linear_scale(0.5)),
                vec4(vec3(0.8, 0.8, 1) * 0.125 * (3.8 + rand0), start_end(1.0, 0.0)),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 0.5)
            );
            break;
        case BLACK_SMOKE:
            attr = Attr(
                linear_motion(
                    vec3(0),
                    vec3(rand2 * 0.02, rand3 * 0.02, 1.0 + rand4 * 0.1)
                ),
                vec3(linear_scale(0.5)),
                vec4(vec3(0.8, 0.8, 1) * 0.125 * (1.8 + rand0), start_end(1.0, 0.0)),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 0.5)
            );
            break;
        case FIRE:
            f_reflect = 0.0; // Fire doesn't reflect light, it emits it
            attr = Attr(
                linear_motion(
                    vec3(0.0),
                    vec3(rand2 * 0.1, rand3 * 0.1, 2.0 + rand4 * 1.0)
                ),
                vec3(1.0),
                vec4(6, 3 + rand5 * 0.3 - 0.8 * percent(), 0.4, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
            );
            break;
        case FIRE_BOWL:
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
            break;
        case GUN_POWDER_SPARK:
            attr = Attr(
                linear_motion(
                    normalize(vec3(rand0, rand1, rand3)) * 0.3,
                    normalize(vec3(rand4, rand5, rand6)) * 4.0 + grav_vel(earth_gravity)
                ),
                vec3(1.0),
                vec4(3.5, 3 + rand7, 0, 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case SHRAPNEL:
            attr = Attr(
                linear_motion(
                    vec3(0),
                    normalize(vec3(rand4, rand5, rand6)) * 20.0 + grav_vel(earth_gravity)
                ),
                vec3(1),
                vec4(vec3(0.25), 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case BIG_SHRAPNEL:
            float brown_color = 0.05 + 0.1 * rand1;
            attr = Attr(
                linear_motion(
                    vec3(0),
                    normalize(vec3(rand4, rand5, rand6)) * 15.0 + grav_vel(earth_gravity)
                ),
                vec3(5 * (1 - percent())),
                vec4(vec3(brown_color, brown_color / 2, 0), 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case FIREWORK_BLUE:
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
            break;
        case FIREWORK_GREEN:
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
            break;
        case FIREWORK_PURPLE:
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
            break;
        case FIREWORK_RED:
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
            break;
        case FIREWORK_WHITE:
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
            break;
        case FIREWORK_YELLOW:
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
            break;
        case LEAF:
            attr = Attr(
                linear_motion(
                    vec3(0),
                    vec3(0, 0, -2)
                ) + vec3(sin(lifetime), sin(lifetime + 0.7), sin(lifetime * 0.5)) * 2.0,
                vec3(4),
                vec4(vec3(0.2 + rand7 * 0.2, 0.2 + (0.25 + rand6 * 0.5) * 0.3, 0) * (0.75 + rand1 * 0.5), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
            );
            break;
        case SNOW:
            float height = mix(-4, 60, pow(start_end(1, 0), 3));
            float wind_speed = (inst_pos.z - 2000) * 0.025;
            vec3 offset = linear_motion(vec3(0), vec3(1, 1, 0) * wind_speed);
            float end_alt = alt_at(start_pos.xy + offset.xy);
            attr = Attr(
                offset + vec3(0, 0, end_alt - start_pos.z + height) + vec3(sin(lifetime), sin(lifetime + 0.7), sin(lifetime * 0.5)) * 3,
                vec3(mix(4, 0, pow(start_end(1, 0), 4))),
                vec4(1),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
            );
            break;
        case FIREFLY:
            float raise = pow(sin(3.1416 * lifetime / inst_lifespan), 0.2);
            attr = Attr(
                vec3(0, 0, raise * 5.0) + vec3(
                    sin(lifetime * 1.0 + rand0) + sin(lifetime * 7.0 + rand3) * 0.3,
                    sin(lifetime * 3.0 + rand1) + sin(lifetime * 8.0 + rand4) * 0.3,
                    sin(lifetime * 2.0 + rand2) + sin(lifetime * 9.0 + rand5) * 0.3
                ),
                vec3(raise),
                vec4(vec3(10.3, 9, 1.5), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
            );
            break;
        case BEE:
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
            break;
        case GROUND_SHOCKWAVE:
            attr = Attr(
                vec3(0.0),
                vec3(11.0, 11.0, (33.0 * rand0 * sin(2.0 * lifetime * 3.14 * 2.0))) / 3,
                vec4(vec3(0.32 + (rand0 * 0.04), 0.22 + (rand1 * 0.03), 0.05 + (rand2 * 0.01)), 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case ENERGY_HEALING:
            f_reflect = 0.0;
            float spiral_radius = start_end(1 - pow(abs(rand5), 5), 1) * length(inst_dir);
            attr = Attr(
                spiral_motion(vec3(0, 0, rand3 + 1), spiral_radius, lifetime, abs(rand0), rand1 * 2 * PI) + vec3(0, 0, rand2),
                vec3(6 * abs(rand4) * (1 - slow_start(2)) * pow(spiral_radius / length(inst_dir), 0.5)),
                vec4(vec3(0, 1.7, 0.7) * 3, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
            );
            break;
        case LIFESTEAL_BEAM:
            f_reflect = 0.0;
            float green_col = 0.8 + 0.8 * sin(tick.x * 5 + lifetime * 5);
            float purple_col = 0.6 + 0.5 * sin(tick.x * 4 - lifetime * 4) - min(max(green_col - 1, 0), 0.3);
            float red_col = 1.15 + 0.1 * sin(tick.x * 3 - lifetime * 3) - min(max(green_col - 1, 0), 0.3) - max(purple_col - 0.5, 0);
            attr = Attr(
                spiral_motion(inst_dir, 0.3 * (floor(2 * rand0 + 0.5) - 0.5) * min(linear_scale(10), 1), lifetime / inst_lifespan, 10.0, inst_time),
                vec3((1.7 - 0.7 * abs(floor(2 * rand0 - 0.5) + 0.5)) * (1.5 + 0.5 * sin(tick.x * 10 - lifetime * 4))),
                vec4(vec3(red_col + purple_col * 0.6, green_col + purple_col * 0.35, purple_col), 1),
                spin_in_axis(inst_dir, tick.z)
            );
            break;
        case ENERGY_NATURE:
            f_reflect = 0.0;
            spiral_radius = start_end(1 - pow(abs(rand5), 5), 1) * length(inst_dir);
            attr = Attr(
                spiral_motion(vec3(0, 0, rand3 + 1), spiral_radius, lifetime, abs(rand0), rand1 * 2 * PI) + vec3(0, 0, rand2),
                vec3(6 * abs(rand4) * (1 - slow_start(2)) * pow(spiral_radius / length(inst_dir), 0.5)),
                vec4(vec3(0, 1.7, 1.3), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
            );
            break;
        case FLAMETHROWER:
            f_reflect = 0.0; // Fire doesn't reflect light, it emits it
            attr = Attr(
                (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (percent() + 2) * 0.1,
                vec3((2.5 * (1 - slow_start(0.2)))),
                vec4(6, 3 + rand5 * 0.6 - 0.8 * percent(), 0.4, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case EXPLOSION:
            f_reflect = 0.0; // Fire doesn't reflect light, it emits it
            attr = Attr(
                inst_dir * ((rand0+1.0)/2 + 0.4) * slow_end(0.25) + 0.3 * grav_vel(earth_gravity),
                vec3((3 * (1 - slow_start(0.1)))),
                vec4(6, 3 + rand5 * 0.3 - 0.8 * percent(), 0.4, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case ICE:
            f_reflect = 0.0; // Ice doesn't reflect to look like magic
            float ice_color = 1.9 + rand5 * 0.3;
            attr = Attr(
                inst_dir * ((rand0+1.0)/2 + 0.4) * slow_end(2.0) + 0.3 * grav_vel(earth_gravity),
                vec3((5 * (1 - slow_start(.1)))),
                vec4(0.8 * ice_color, 0.9 * ice_color, ice_color, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case FIRE_SHOCKWAVE:
            f_reflect = 0.0; // Fire doesn't reflect light, it emits it
            attr = Attr(
                vec3(rand0, rand1, lifetime * 10 + rand2),
                vec3((5 * (1 - slow_start(0.5)))),
                vec4(6, 3 + rand5 * 0.6 - 0.8 * percent(), 0.4, 1),
                spin_in_axis(vec3(rand3, rand4, rand5), rand6)
            );
            break;
        case CULTIST_FLAME:
            f_reflect = 0.0; // Fire doesn't reflect light, it emits it
            float purp_color = 0.9 + 0.3 * rand3;
            attr = Attr(
                (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (percent() + 2) * 0.1,
                vec3((3.5 * (1 - slow_start(0.2)))),
                vec4(purp_color, 0.0, purp_color, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case STATIC_SMOKE:
            attr = Attr(
                vec3(0),
                vec3((0.5 * (1 - slow_start(0.8)))),
                vec4(1.0),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9)
            );
            break;
        case BLOOD:
            attr = Attr(
                linear_motion(
                    vec3(0),
                    normalize(vec3(rand4, rand5, rand6)) * 5.0 + grav_vel(earth_gravity)
                ),
                vec3((2.0 * (1 - slow_start(0.8)))),
                vec4(1, 0, 0, 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case ENRAGED:
            f_reflect = 0.0;
            float red_color = 1.2 + 0.3 * rand3;
            attr = Attr(
                (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (percent() + 2) * 0.1,
                vec3((3.5 * (1 - slow_start(0.2)))),
                vec4(red_color, 0.0, 0.0, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case LASER:
            f_reflect = 0.0;
            vec3 perp_axis = normalize(cross(inst_dir, vec3(0.0, 0.0, 1.0)));
            offset = vec3(0.0);
            if (rand0 > 0.0) {
                offset = perp_axis * 0.5;
            } else {
                offset = perp_axis * -0.5;
            }
            attr = Attr(
                inst_dir * percent() + offset,
                vec3(1.0, 1.0, 50.0),
                vec4(vec3(2.0, 0.0, 0.0), 1),
                spin_in_axis(perp_axis, asin(inst_dir.z / length(inst_dir)) + PI / 2.0)
            );
            break;
        case BUBBLES:
            f_reflect = 0.0; // Magic water doesn't reflect light, it emits it
            float blue_color = 1.5 + 0.2 * rand3 + 1.5 * max(floor(rand4 + 0.3), 0.0);
            float size = 8.0 * (1 - slow_start(0.1)) * slow_end(0.15);
            attr = Attr(
                (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (percent() + 2) * 0.1,
                vec3(size),
                vec4(0.5 * blue_color, 0.75 * blue_color, blue_color, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case WATER:
            f_reflect = 0.0; // Magic water doesn't reflect light, it emits it
            blue_color = 1.25 + 0.2 * rand3 + 1.75 * max(floor(rand4 + 0.15), 0.0);
            size = 8.0 * (1 - slow_start(0.1)) * slow_end(0.15);
            attr = Attr(
                (inst_dir * slow_end(0.2)) + vec3(rand0, rand1, rand2) * 0.5,
                vec3(size),
                vec4(0.5 * blue_color, 0.9 * blue_color, blue_color, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 5 + 3 * rand9)
            );
            break;
        case ICE_SPIKES:
            f_reflect = 0.0; // Ice doesn't reflect to look like magic
            ice_color = 1.7 + rand5 * 0.2;
            attr = Attr(
                vec3(0.0),
                vec3(11.0, 11.0, 11.0 * length(inst_dir) * 2.0 * (0.5 - abs(0.5 - slow_end(0.5)))) / 3,
                vec4(0.8 * ice_color, 0.9 * ice_color, ice_color, 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case DRIP:
            attr = Attr(
                linear_motion(
                    vec3(0),
                    normalize(vec3(rand4, rand5, rand6))  + grav_vel(earth_gravity)
                ),
                vec3((2.0 * (1 - slow_start(0.2)))),
                vec4(1, 1, 0, 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case TORNADO:
            f_reflect = 0.0;
            attr = Attr(
                spiral_motion(vec3(0, 0, 5), abs(rand0) + abs(rand1) * percent() * 3.0, percent(), 15.0 * abs(rand2), rand3),
                vec3((2.5 * (1 - slow_start(0.05)))),
                vec4(vec3(1.2 + 0.5 * percent()), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case DEATH:
            f_reflect = 0.0;
            attr = Attr(
                linear_motion(
                    vec3(0),
                    vec3(rand2 * 0.02, rand3 * 0.02, 2.0 + rand4 * 0.6)
                ),
                vec3((1.2 * (1 - slow_start(.1)))),
                vec4(vec3(1.2 + 0.5 * percent()), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case ENERGY_BUFFING:
            f_reflect = 0.0;
            spiral_radius = start_end(1 - pow(abs(rand5), 5), 1) * length(inst_dir);
            attr = Attr(
                spiral_motion(vec3(0, 0, rand3 + 1), spiral_radius, lifetime, abs(rand0), rand1 * 2 * PI) + vec3(0, 0, rand2),
                vec3(6 * abs(rand4) * (1 - slow_start(2)) * pow(spiral_radius / length(inst_dir), 0.5)),
                vec4(vec3(1.4), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3)
            );
            break;
        case WEB_STRAND:
            f_reflect = 0.0;
            perp_axis = normalize(cross(inst_dir, vec3(0.0, 0.0, 1.0)));
            attr = Attr(
                inst_dir * percent(),
                vec3(1.0, 1.0, 50.0),
                vec4(vec3(2.0), 1),
                spin_in_axis(perp_axis, asin(inst_dir.z / length(inst_dir)) + PI / 2.0)
            );
            break;
        case LIGHTNING:
            f_reflect = 0.0;
            perp_axis = normalize(cross(inst_dir, vec3(0.0, 0.0, 1.0)));
            float z = inst_dir.z * (percent() - 1.0);
            vec3 start_off = vec3(abs(fract(vec3(vec2(z) * vec2(0.015, 0.01), 0)) - 0.5) * z * 0.4);
            attr = Attr(
                inst_dir * percent() + start_off,
                vec3(max(3.0, 0.05 * length(start_pos + inst_dir * percent()))),
                vec4(10.0, 20.0, 50.0, 1.0),// * (1.0 - length(inst_dir) * 0.1),
                identity()//spin_in_axis(perp_axis, asin(inst_dir.z / length(inst_dir)) + PI / 2.0)
            );
            break;
        case STEAM:
            f_reflect = 0.0; // Magic steam doesn't reflect light, it emits it
            float steam_size = 8.0 * (1 - slow_start(0.1)) * slow_end(0.15);
            attr = Attr(
                (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (percent() + 2) * 0.1,
                vec3(steam_size),
                vec4(vec3(0.7, 2.7, 1.3), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case BARRELORGAN:
            attr = Attr(
                linear_motion(
                    vec3(rand0 * 0.25, rand1 * 0.25, 1.7 + rand5),
                    vec3(rand2 * 0.1, rand3 * 0.1, 1.0 + rand4 * 0.5)
                ),
                vec3(exp_scale(-0.2)) * rand0,
                vec4(vec3(0.7, 2.7, 1.3), 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case POTION_SICKNESS:
            attr = Attr(
                quadratic_bezier_motion(
                    vec3(0.0),
                    vec3(inst_dir.xy, 0.0),
                    inst_dir
                ),
                vec3((2.0 * (1 - slow_start(0.8)))),
                vec4(0.075, 0.625, 0, 1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case GIGA_SNOW:
            f_reflect = 0.0;
            attr = Attr(
                (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (percent() + 2) * 0.1,
                vec3((3.5 * (1 - slow_start(0.2)))),
                vec4(vec3(2, 2, 2), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        default:
            attr = Attr(
                linear_motion(
                    vec3(rand0 * 0.25, rand1 * 0.25, 1.7 + rand5),
                    vec3(rand2 * 0.1, rand3 * 0.1, 1.0 + rand4 * 0.5)
                ),
                vec3(exp_scale(-0.2)) * rand0,
                vec4(1),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
    }

    // Temporary: use shrinking particles as a substitute for fading ones
    attr.scale *= pow(attr.col.a, 0.25);

    f_pos = start_pos + (v_pos * attr.scale * SCALE * mat3(attr.rot) + attr.offs);

    #ifdef EXPERIMENTAL_CURVEDWORLD
        f_pos.z -= pow(distance(f_pos.xy + focus_off.xy, focus_pos.xy + focus_off.xy) * 0.05, 2);
    #endif

    // First 3 normals are negative, next 3 are positive
    // TODO: Make particle normals match orientation
    vec4 normals[6] = vec4[](vec4(-1,0,0,0), vec4(1,0,0,0), vec4(0,-1,0,0), vec4(0,1,0,0), vec4(0,0,-1,0), vec4(0,0,1,0));
    f_norm =
        // inst_pos *
        normalize(((normals[(v_norm_ao >> 0) & 0x7u]) * attr.rot).xyz);

    //vec3 col = vec3((uvec3(v_col) >> uvec3(0, 8, 16)) & uvec3(0xFFu)) / 255.0;
    f_col = vec4(attr.col.rgb, attr.col.a);

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
}
