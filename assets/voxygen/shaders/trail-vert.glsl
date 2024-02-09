#version 440 core

#include <globals.glsl>

layout(location = 0) in vec3 v_pos;

void main() {
    gl_Position = all_mat * vec4(v_pos - focus_off.xyz, 1);
}
