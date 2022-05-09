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
layout(location = 1) in vec3 v_norm;
layout(location = 2) in vec3 v_col;
layout(location = 3) in vec3 inst_pos;

layout(location = 0) out vec3 f_pos;
layout(location = 1) out vec3 f_norm;
layout(location = 2) out vec4 f_col;

void main() {
    f_pos = inst_pos + v_pos - focus_off.xyz;

    float pull_down = 1.0 / pow(distance(focus_pos.xy, f_pos.xy) / (view_distance.x * 0.95), 50.0);
    f_pos.z -= pull_down;

    f_norm = v_norm;
    f_col = vec4(vec3(0.01, 0.04, 0.0) * 1, 1.0);//vec4(v_col, 1.0);

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
}
