#version 460 core

#include <globals.glsl>

layout (location = 0)
in vec3 v_pos;

layout (std140, set = 1, binding = 0)
uniform u_locals {
    vec4 w_pos;
    vec4 w_color;
};

layout (location = 0)
out vec4 f_color;

void main() {
    f_color = w_color;
    gl_Position = all_mat * vec4((v_pos + w_pos.xyz) - focus_off.xyz, 1);
}
