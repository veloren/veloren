#version 420 core

#include <constants.glsl>

#define LIGHTING_TYPE (LIGHTING_TYPE_TRANSMISSION | LIGHTING_TYPE_REFLECTION)

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_SPECULAR

#if (FLUID_MODE == FLUID_MODE_LOW)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE >= FLUID_MODE_MEDIUM)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <srgb.glsl>
#include <random.glsl>

layout(location = 0) in uint v_pos_norm;
layout(location = 1) in uint v_vel;
// in uint v_col_light;

layout(std140, set = 2, binding = 0)
uniform u_locals {
    vec3 model_offs;
    float load_time;
    ivec4 atlas_offs;
};

// struct ShadowLocals {
//     mat4 shadowMatrices;
//     mat4 texture_mat;
// };
//
// layout (std140)
// uniform u_light_shadows {
//     ShadowLocals shadowMats[/*MAX_LAYER_FACES*/192];
// };

layout(location = 0) out vec3 f_pos;
layout(location = 1) flat out uint f_pos_norm;
layout(location = 2) out vec2 f_vel;
// out vec3 f_col;
// out float f_light;
// out vec3 light_pos[2];

const float EXTRA_NEG_Z = 65536.0/*65536.1*/;

void main() {
    f_pos = vec3(v_pos_norm & 0x3Fu, (v_pos_norm >> 6) & 0x3Fu, float((v_pos_norm >> 12) & 0x1FFFFu) - EXTRA_NEG_Z) + model_offs - focus_off.xyz;
    f_vel = vec2(
        (float(v_vel & 0xFFFFu) - 32768.0) / 1000.0,
        (float((v_vel >> 16u) & 0xFFFFu) - 32768.0) / 1000.0
    );

    // f_pos.z -= 250.0 * (1.0 - min(1.0001 - 0.02 / pow(tick.x - load_time, 10.0), 1.0));
    // f_pos.z -= min(32.0, 25.0 * pow(distance(focus_pos.xy, f_pos.xy) / view_distance.x, 20.0));

    // Terrain 'pop-in' effect
    #ifndef EXPERIMENTAL_BAREMINIMUM
        #ifndef EXPERIMENTAL_NOTERRAINPOP
            f_pos.z -= 250.0 * (1.0 - min(1.0001 - 0.02 / pow(tick.x - load_time, 10.0), 1.0));
            // f_pos.z -= min(32.0, 25.0 * pow(distance(focus_pos.xy, f_pos.xy) / view_distance.x, 20.0));
        #endif
    #endif

    float pull_down = pow(distance(focus_pos.xy, f_pos.xy) / (view_distance.x * 0.95), 20.0) * 0.7;
    //f_pos.z -= pull_down;

    #ifdef EXPERIMENTAL_CURVEDWORLD
        f_pos.z -= pow(distance(f_pos.xy + focus_off.xy, focus_pos.xy + focus_off.xy) * 0.05, 2);
    #endif

    // Small waves
    // f_pos.xy += 0.01; // Avoid z-fighting
    // f_pos.x += 0.1 * sin(tick.x / 60 * hash(vec4(f_pos.xyz, 1.0)));
    // f_pos.y += 0.1 * sin(tick.x / 60 * hash(vec4(f_pos.xyz, 2.0)));
#if (FLUID_MODE >= FLUID_MODE_MEDIUM)
    // f_pos.z -= 0.1 + 0.1 * (sin(tick.x/* / 60.0*/* 2.0 + f_pos.x * 2.0 + f_pos.y * 2.0) + 1.0) * 0.5;
#endif

    /* f_col = vec3(
        float((v_col_light >>  8) & 0xFFu),
        float((v_col_light >> 16) & 0xFFu),
        float((v_col_light >> 24) & 0xFFu)
    ) / 255.0;

    f_light = float(v_col_light & 0xFFu) / 255.0; */
    /* for (uint i = 0u; i < light_shadow_count.z; ++i) {
        light_pos[i] = vec3(shadowMats[i].texture_mat * vec4(f_pos, 1.0));
    } */

    f_pos_norm = v_pos_norm;

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
    // gl_Position.z = -gl_Position.z / gl_Position.w;
    // gl_Position.z = -gl_Position.z / 100.0;
    // gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
