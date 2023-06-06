#version 420 core

#include <constants.glsl>

#define FIGURE_SHADER

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>

layout(location = 0) in vec3 v_pos;
layout(location = 1) in vec3 v_norm;

layout (std140, set = 2, binding = 0)
uniform u_locals {
    vec4 pos_a;
    vec4 pos_b;
};

layout(location = 0) out vec3 f_pos;
layout(location = 1) out vec3 f_norm;

void main() {
    vec3 pos = pos_a.xyz + v_pos * vec3(1, 1, 5);

    f_pos = pos + focus_pos.xyz;

    #ifdef EXPERIMENTAL_CURVEDWORLD
        f_pos.z -= pow(distance(f_pos.xy + focus_off.xy, focus_pos.xy + focus_off.xy) * 0.05, 2);
    #endif

    f_norm = v_norm;

    gl_Position = all_mat * vec4(f_pos, 1);
}
