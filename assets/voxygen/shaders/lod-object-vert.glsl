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
layout(location = 1) in vec3 v_norm;
layout(location = 2) in vec3 v_col;
layout(location = 3) in vec3 inst_pos;
layout(location = 4) in uvec3 inst_col;
layout(location = 5) in uint inst_flags;

layout(location = 0) out vec3 f_pos;
layout(location = 1) out vec3 f_norm;
layout(location = 2) out vec4 f_col;
layout(location = 3) out vec3 model_pos;
layout(location = 4) flat out uint f_flags;

void main() {
    vec3 obj_pos = inst_pos - focus_off.xyz;
    f_pos = obj_pos + v_pos;
    model_pos = v_pos;

    float pull_down = 1.0 / pow(distance(focus_pos.xy, obj_pos.xy) / (view_distance.x * 0.95), 150.0);
    #ifdef EXPERIMENTAL_TERRAINPOP
        f_pos.z -= pull_down;
    #else
        f_pos.z -= step(0.1, pull_down) * 10000.0;
    #endif

    #ifdef EXPERIMENTAL_CURVEDWORLD
        f_pos.z -= pow(distance(f_pos.xy + focus_off.xy, focus_pos.xy + focus_off.xy) * 0.05, 2);
    #endif

    f_norm = v_norm;
    f_col = vec4(vec3(inst_col) * (1.0 / 255.0) * v_col * (hash(inst_pos.xyxy) * 0.4 + 0.6), 1.0);
    f_flags = inst_flags;

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
}
