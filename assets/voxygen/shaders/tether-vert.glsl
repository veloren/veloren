#version 420 core

#include <constants.glsl>

#define FIGURE_SHADER

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <sky.glsl>

layout(location = 0) in vec3 v_pos;
layout(location = 1) in vec3 v_norm;

layout (std140, set = 2, binding = 0)
uniform u_locals {
    vec4 pos_a;
    vec4 pos_b;
    float tether_length;
};

layout(location = 0) out vec3 f_pos;
layout(location = 1) out vec3 f_norm;
layout(location = 2) out vec3 m_pos;

void main() {
    m_pos = v_pos;

    vec3 rz = normalize(pos_b.xyz - pos_a.xyz);
    vec3 rx = normalize(cross(vec3(0, 0, 1), rz));
    vec3 ry = normalize(cross(rz, rx));
    float dist = distance(pos_a.xyz, pos_b.xyz);
    vec3 pos = pos_a.xyz + (rx * v_pos.x + ry * v_pos.y) * 0.1 + rz * v_pos.z * dist;
    vec2 ideal_wind_sway = wind_vel * vec2(
        wind_wave(pos.y * 1.5, 2.9, wind_vel.x, wind_vel.y),
        wind_wave(pos.x * 1.5, 3.1, wind_vel.y, wind_vel.x)
    );
    float dip = (1 - pow(abs(v_pos.z - 0.5) * 2.0, 2)) * max(tether_length - dist, 0.0);
    pos += vec3(ideal_wind_sway * min(pow(dip, 2), 0.005), -0.5 * dip);

    f_pos = pos + focus_pos.xyz;

    #ifdef EXPERIMENTAL_CURVEDWORLD
        f_pos.z -= pow(distance(f_pos.xy + focus_off.xy, focus_pos.xy + focus_off.xy) * 0.05, 2);
    #endif

    f_norm = rx * v_norm.x + ry * v_norm.y + rz * v_norm.z;

    gl_Position = all_mat * vec4(f_pos, 1);
}
