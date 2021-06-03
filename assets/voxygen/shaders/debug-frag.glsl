#version 420 core

#include <globals.glsl>

layout (location = 0)
in vec4 f_color;

layout (std140, set = 1, binding = 0)
uniform u_locals {
    vec4 w_pos;
    vec4 w_color;
};

layout (location = 0)
out vec4 tgt_color;

void main() {
    tgt_color = f_color;
}
