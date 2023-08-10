#version 440 core

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
const int CYCLOPS_CHARGE = 43;
const int PORTAL_FIZZ = 45;
const int INK = 46;
const int WHIRLWIND = 47;
const int FIERY_BURST = 48;
const int FIERY_BURST_VORTEX = 49;
const int FIERY_BURST_SPARKS = 50;
const int FIERY_BURST_ASH = 51;
const int FIERY_TORNADO = 52;
const int PHOENIX_CLOUD = 53;
const int FIERY_DROPLET_TRACE = 54;
const int ENERGY_PHOENIX = 55;
const int PHOENIX_BEAM = 56;
const int PHOENIX_BUILD_UP_AIM = 57;
const int CLAY_SHRAPNEL = 58;
const int AIRFLOW = 47;

// meters per second squared (acceleration)
const float earth_gravity = 9.807;

struct Attr {
    vec3 offs;
    vec3 scale;
    vec4 col;
    mat4 rot;
};

float lifetime = time_since(inst_time);

// Retrieves inst_time, repeating over a period. This will be consistent
// over a time overflow.
float loop_inst_time(float period, float scale) {
    if (tick.x < inst_time) {
        return mod(mod(tick_overflow * scale, period) + inst_time * scale, period);
    } else {
        return mod(inst_time * scale, period);
    }
}

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
            float green_col = 0.8 + 0.8 * sin(tick_loop(2 * PI, 5, lifetime * 5));
            float purple_col = 0.6 + 0.5 * sin(loop_inst_time(2 * PI, 4)) - min(max(green_col - 1, 0), 0.3);
            float red_col = 1.15 + 0.1 * sin(loop_inst_time(2 * PI, 3)) - min(max(green_col - 1, 0), 0.3) - max(purple_col - 0.5, 0);
            attr = Attr(
                spiral_motion(inst_dir, 0.3 * (floor(2 * rand0 + 0.5) - 0.5) * min(linear_scale(10), 1), lifetime / inst_lifespan, 10.0, loop_inst_time(2.0 * PI, 1.0)),
                vec3((1.7 - 0.7 * abs(floor(2 * rand0 - 0.5) + 0.5)) * (1.5 + 0.5 * sin(tick_loop(2 * PI, 10, -lifetime * 4)))),
                vec4(vec3(red_col + purple_col * 0.6, green_col + purple_col * 0.35, purple_col), 1),
                spin_in_axis(inst_dir, tick_loop(2 * PI))
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
        case CYCLOPS_CHARGE:
            f_reflect = 0.0;
            float burn_size = 8.0 * (1 - slow_start(0.1)) * slow_end(0.15);
            attr = Attr(
                (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (percent() + 2) * 0.1,
                vec3(burn_size),
                vec4(vec3(6.9, 0.0, 0.0), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case PORTAL_FIZZ:
            attr = Attr(
                inst_dir * (0.7 + pow(percent(), 5)) + vec3(
                    sin(lifetime * 1.25 + rand0 * 10) + sin(lifetime * 1.3 + rand3 * 10),
                    sin(lifetime * 1.2 + rand1 * 10) + sin(lifetime * 1.4 + rand4 * 10),
                    sin(lifetime * 5 + rand2)
                ) * 0.03,
                vec3(pow(1.0 - abs(percent() - 0.5) * 2.0, 0.2)),
                mix(
                    vec4(mix(vec3(0.4, 0.8, 0.2), vec3(5, 10, 2), pow(percent(), 2)), 1),
                    vec4(mix(vec3(0.6, 0.2, 0.8), vec3(9, 2, 10), pow(percent(), 2)), 1),
                    clamp((dot(normalize(focus_pos.xyz - start_pos), inst_dir) - 0.25) * 3.0, 0.0, 1.0)
                ),
                /* vec4(vec3(1.8 - percent() * 2, 0.4 + percent() * 2, 5.0 + rand6), 1), */
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3 + lifetime * 5)
            );
            break;
        case INK:
            f_reflect = 0.0; // Magic water doesn't reflect light, it emits it
            float black_color = 0.3 + 0.2 * rand3 + 0.3 * max(floor(rand4 + 0.3), 0.0);
            float ink_size = 8.0 * (1 - slow_start(0.1)) * slow_end(0.15);
            attr = Attr(
                (inst_dir * slow_end(1.5)) + vec3(rand0, rand1, rand2) * (percent() + 2) * 0.1,
                vec3(ink_size),
                vec4(0.5 * black_color, 0.75 * black_color, black_color, 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case WHIRLWIND:
            f_reflect = 0.0;
            attr = Attr(
                spiral_motion(vec3(0, 0, 3), abs(rand0) * 3 + percent() * 20.5, percent(), -8.0 + (rand0 * 3), rand1 * 360.),
                vec3((-2.5 * (1 - slow_start(0.05)))),
                vec4(vec3(1.3, 1.8, 2), 1),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case FIERY_BURST:
            f_reflect = 0.0;
            float fiery_radius = start_end(1.0 - pow(abs(rand5), 5.0), 1.0) * length(inst_dir);
            float fiery_color1 = (7.0 + 1.0 * percent()) * min(1.0, percent() * 4.0) * 1.5;
            float fiery_color2 = (4.0 - 2.0 * percent() + 1.3 * rand5 * slow_end(0.0)) * min(1.0, percent() * 4.0) * 1.3;
            float fiery_color3 = 1.0 + 0.3 * percent();
            attr = Attr(
                spiral_motion(
                    vec3(
                        0.0,
                        0.0,
                        (rand3 + 1.0)
                        * max(
                            ((percent() * 8.0) * (1.0 - step(0.2, percent()))),
                            ((2.0 * (1.0 - percent())) * (step(0.2, percent())))
                        )
                    ),
                    fiery_radius,
                    lifetime,
                    max(0.1, step(0.6, percent())) * 3.0 * abs(rand0),
                    rand1 * 2.0 * PI) + vec3(0.0, 0.0, rand2),
                vec3(6.0 * abs(rand4) * (1.0 - slow_start(2.0)) * pow(fiery_radius / length(inst_dir), 0.5)),
                vec4(fiery_color1, fiery_color2, fiery_color3, slow_end(0.4)),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3.0)
            );
            break;
        case FIERY_BURST_VORTEX:
            f_reflect = 0.0;
            float fiery_vortex_color1 = (min(1, percent() * 2) * (5 + 1 * percent() + 1 * slow_end(0)) * 1.5);
            float fiery_vortex_color2 = (min(1, percent() * 2) * (4 - 2.4 * percent() + 1.3 * rand5 * slow_end(0)) * 1.3);
            float fiery_vortex_color3 = 0;
            attr = Attr(
                spiral_motion(
                    vec3(
                        0,
                        0,
                        (0 + 0.5 * rand4 ) + 4.0
                            * max(
                                ((percent() * 8) * (1 - step(0.2, percent()))), // first 20% of lifetime particle moves up, then goes down
                                ((2 * (1 - percent())) * (step(0.2, percent())))// to avoid tearing multi should have same proportion as edge(here: 8 before, 2 after)
                            )
                    ),
                    abs(rand0) + 0.5 * 10 * percent(),
                    percent(),
                    10.0 * abs(rand2),
                    rand3),
                vec3((2.5 * (1 - slow_start(0.05)))),
                vec4(fiery_vortex_color1, fiery_vortex_color2, fiery_vortex_color3, start_end(0.5, 1.5) * abs(rand2)),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case FIERY_BURST_SPARKS:
            f_reflect = 0.0;
            // sparks should flicker, so it stops glowing for 18% of time 4 times per second, same thing used in 4th float of RGBA vector
            float fiery_sparks_color1 = 2 + 1 * rand2 + 2 * step(0.18, fract(tick.x*4));
            float fiery_sparks_color2 = 4 + 1 * rand2 + 4 * step(0.18, fract(tick.x*4));
            float fiery_sparks_color3 = 4 + 6 * step(0.18, fract(tick.x*4));
            attr = Attr(
                spiral_motion(vec3(0, 0, 5), abs(rand0) + abs(rand1) * percent() * 4.0, percent(), 8.0 * abs(rand2), rand3),
                vec3((2.5 * (1 - slow_start(0.05)))),
                vec4(fiery_sparks_color1, fiery_sparks_color2, fiery_sparks_color3, 0.5 + 0.5 * step(0.18, fract(tick.x*4))),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case FIERY_BURST_ASH:
            f_reflect = 0.0;
            /// inst_dir holds info about:
            /// .x: radius of random spawn
            float fiery_ash_rand_rad = inst_dir.x;
            /// .y:
            ///     in fract:   relative time of "setting on fire"
            ///     in int:     radius of curve
            float fiery_ash_radius = floor(inst_dir.y);
            float fiery_ash_edge = inst_dir.y - fiery_ash_radius;
            /// .z: height of the flight
            float fiery_ash_height = inst_dir.z;
            // {FOR PHOENIX "from the ashes"}sets ash on fire at 0.4 of lifetime, then makes it lose glow, representing losing heat
            float fiery_ash_color1 = (2 + 1 * percent() * slow_end(0))
                            * (max(
                                1,
                                8 * step(fiery_ash_edge, percent()) * (1.4 - percent()))
                            );
            float fiery_ash_color2 = (2 - 1 * percent() + 0.3 * abs(rand5) * slow_end(0.5))
                            * (max(
                                1,
                                6.5 * step(fiery_ash_edge, percent()) * (1.4 - percent()))
                            );
            float fiery_ash_color3 = 1.5;
            attr = Attr(
                spiral_motion(
                    vec3(
                        0.0,
                        0.0,
                        fiery_ash_height// {FOR PHOENIX "from the ashes"} 8.58
                    ),
                    abs(rand0 / 2.0 + 1.0)
                        * max(1.0, ((percent() * fiery_ash_radius * 0.8) * (1.0 - step(0.2, percent())))) // part of lifetime particle moves to periphery
                        * max(1.0, (fiery_ash_radius * 0.2 * (1.0 - percent()) * (step(0.2, percent())))),// then back to center
                    percent(),
                    6.0 * abs(rand2),
                    rand3 * 5.0
                )
                + vec3((rand6 + rand5) * fiery_ash_rand_rad, (rand8 + rand3) * fiery_ash_rand_rad, abs(rand0)),//makes it apear randomly above base animation (Fiery Burst)
                vec3((2.5 * (1 - slow_start(0.0)))),
                vec4(fiery_ash_color1, fiery_ash_color2, fiery_ash_color3, abs(rand2) * slow_end(0.3)),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case FIERY_TORNADO:
            f_reflect = 0.0;
            float fiery_tornado_color1 = (2.6 + 0.5 * percent())
                            * 4.0 * max(0.5, percent() * 1.2);
            float fiery_tornado_color2 = (1.7 - 0.6 * pow(1.0 - percent(), 2.0) + 0.3 * abs(rand5))
                            * 2.0 * max(0.45, percent() * 1.2);
            float fiery_tornado_color3 = 1.5 * max(0.6, percent());
            attr = Attr(
                spiral_motion(vec3(0, 0, 6.0 + rand3 * 1.5), abs(rand0) + abs(rand1) * percent() * 3.0, percent(), 15.0 * abs(rand2), -inst_time),
                vec3((2.5 * (1 - slow_start(0.05)))),
                vec4(fiery_tornado_color1, fiery_tornado_color2, fiery_tornado_color3, 0.5),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10 + 3 * rand9)
            );
            break;
        case PHOENIX_CLOUD:
            float PC_spin =  floor(inst_dir.x);
            float refl = floor(inst_dir.y);
            float PC_size = floor(inst_dir.z);
            //best is 0.4 - reflects some light but only part as 
            f_reflect = refl * 0.1; 
            // modifies by + 5% to -15%, if color is less than 0.5 it will get from +10% to +25% to it's value
            float PC_rand_color_factor = rand0 * 0.05;
            float PC_R = inst_dir.x - PC_spin;
                PC_R += PC_R * PC_rand_color_factor * step(0.05, PC_R) * -abs(PC_rand_color_factor * 2.0)
                    + PC_R * (1.0 - step(0.05, PC_R)) * max(abs(PC_rand_color_factor), 0.02) * 5.0;
            float PC_G = inst_dir.y - refl;
                PC_G += PC_G * PC_rand_color_factor * step(0.05, PC_G) * -abs(PC_rand_color_factor * 2.0)
                    + PC_G * (1.0 - step(0.05, PC_G)) * max(abs(PC_rand_color_factor), 0.02) * 5.0;
            float PC_B = inst_dir.z - PC_size;
                PC_B += PC_B * PC_rand_color_factor * step(0.05, PC_B) * -abs(PC_rand_color_factor * 2.0)
                    + PC_B * (1.0 - step(0.05, PC_B)) * max(abs(PC_rand_color_factor), 0.02) * 5.0;
            attr = Attr(
                linear_motion(
                    vec3(0.0, 0.0, 0.0),
                    vec3(rand4, rand5, rand6 * 2.5)
                ),
                vec3(8.0 * min(percent() * 3.0, 1.0) * min((1.0 - percent()) * 2.0, 1.0)),
                vec4(
                    PC_R,
                    PC_G,
                    PC_B,
                    PC_size * 1.2) * 10.0,
                spin_in_axis(vec3(rand6 + rand5, rand7 + rand9, rand8 + rand2), percent() * PC_spin)
            );
            break;
        case FIERY_DROPLET_TRACE:
            float m_r = 4.0;
            f_reflect = 0.0; // Fire doesn't reflect light, it emits it
            float prcnt = percent(); //idk if compiler would optimize it or not but as we have a lot of those particles... i'll just try
            float droplet_color1 = 1 * (5 + 1 * prcnt + 1 * slow_end(0)) * 1.5;
            float droplet_color2 = 1 * (4 - 2.4 * prcnt + 1.3 * rand5 * slow_end(0)) * 1.3;
            float droplet_color3 = 0;
            attr = Attr(
                quadratic_bezier_motion(
                    vec3(0.0),
                    vec3(m_r * rand0, m_r * rand1, 0.0),
                    vec3(m_r * rand0, m_r * rand1, 4.0)
                ),
                vec3(1),
                vec4(droplet_color1,
                    droplet_color2,
                    droplet_color3,
                    1 * prcnt * (1 - step(0.5, prcnt)) + (1 - prcnt) * (step(0.5, prcnt))),
                spin_in_axis(vec3(1,0,0),0)
            );
            break;
        case ENERGY_PHOENIX:
            f_reflect = 0.0;
            float fiery_r = (2 + 1 * percent() * slow_end(0))
                            * 6 * (1.4 - percent());
            float fiery_g = (2 - 1 * percent() + 0.3 * abs(rand5) * slow_end(0.5))
                            * 4.5 * (1.4 - percent());
            float fiery_b = 1.5;
            spiral_radius = length(inst_dir);
            attr = Attr(
                spiral_motion(vec3(0.0, 0.0, 0.01), spiral_radius + abs(rand1), lifetime / 0.5, abs(rand0), rand1 * 2.0 * PI) + vec3(0.0, 0.0, rand2),
                vec3(6.0 * abs(rand4) * (1 - slow_start(2.0))),
                vec4(vec3(fiery_r, fiery_g, fiery_b), 1.0),
                spin_in_axis(vec3(rand6, rand7, rand8), rand9 * 3.0)
            );
            break;
        case PHOENIX_BEAM:
            f_reflect = 0.0; // Fire doesn't reflect light, it emits it
            float beam_r = 6.0 - (4.0 * percent()) + 15.0 * fract(percent() * 4 + rand0 * rand0) * (1 - percent());
            float beam_g = 2.0 + 6.6 * fract(percent() * 4 + rand0 * rand0) * (1 - percent());
            float beam_b = 1.4;

            vec3 factor_rand = vec3((rand0 * 0.2) * (rand5 * 0.1) + rand6 * 0.9, (rand1 * 0.2) * (rand4 * 0.1) + rand7 * 0.9, (rand2 * 0.2) * (rand3 * 0.1) + rand8 * 0.9);
            start_pos += factor_rand + normalize(inst_dir) * 0.6;
            attr = Attr(
                spiral_motion(inst_dir - factor_rand * 0.4, 0.3 * ((rand2 + 0.5) * 5.5) * (1.0 - min(linear_scale(1.5), 1.0)), lifetime / inst_lifespan, 24.0, -inst_time * 8.0),
                vec3((2.5 * (1 - slow_start(0.2)))),
                vec4(beam_r, beam_g, beam_b, 1.0),
                spin_in_axis(vec3(rand6, rand7, rand8), percent() * 10.0 + 3.0 * rand9)
            );
            break;
        case PHOENIX_BUILD_UP_AIM:

            f_reflect = 0.0; // Fire doesn't reflect light, it emits it

            float perc_t = percent(); // in case compiler wont optimize, idk

            float aim_r = rand0 * 0.25 + 3.0 + 4.5 * perc_t * (1 - step(0.79, perc_t)) + 8.0 * step(0.81, perc_t) * perc_t;
            float aim_g = rand0 * 0.25 + 2.0 - 1.0 * perc_t * (1 - step(0.79, perc_t)) + 2.0 * step(0.81, perc_t) * perc_t;
            float aim_b = 1.4 * ((1 - perc_t) + step(0.74, perc_t));


            vec3 dir_aim = inst_dir * 1.0;
            vec3 rand_pos_aim = (cross(
                (1.0 - 2.0 * step(0.0, rand2)) * normalize(inst_dir),
                vec3(0.0, 0.0, 1.0)));

            vec3 rand_fact = vec3(rand1 * 1, rand0 * 1, rand2 * 1);
            start_pos += vec3(0.0, 0.0, 5.0) + rand_fact;
            attr = Attr(
                spiral_motion(
                    inst_dir + vec3(0.0, 0.0, -(6.0 - 3.0 * pow(perc_t, 2.5))) - rand_fact,
                    1.2 * rand9 * max(1.0 - perc_t, 0.0),
                    perc_t,
                    6.0,
                    inst_time * 8.0),
                vec3((1.9 * (1 - slow_start(0.2)))),
                vec4(aim_r, aim_g, aim_b, 1.0),
                spin_in_axis(vec3(rand6, rand7, rand8), perc_t * 10.0 + 3.0 * rand9)
            );
            break;
        case CLAY_SHRAPNEL:
            float clay_color = 0.025 + 0.02 * rand1;
            attr = Attr(
                linear_motion(
                    vec3(0),
                    normalize(vec3(rand4, rand5, rand6)) * 15.0 + grav_vel(earth_gravity)
                ),
                vec3(5 * (1 - percent())),
                vec4(vec3(clay_color * 3, clay_color * 2, clay_color), 1),
                spin_in_axis(vec3(1,0,0),0)
        case AIRFLOW:
            perp_axis = normalize(cross(inst_dir, vec3(0.0, 0.0, 1.0)));
            attr = Attr(
                inst_dir * 0.2 * length(inst_dir) * percent() + inst_dir * percent() * 0.08,
                vec3(0.03 * length(inst_dir), 0.03 * length(inst_dir), 20.0 * length(inst_dir) * percent() * (1 - percent())),
                vec4(1.1, 1.1, 1.1, 0.3),
                spin_in_axis(perp_axis, asin(inst_dir.z / length(inst_dir)) + PI / 2.0)
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
