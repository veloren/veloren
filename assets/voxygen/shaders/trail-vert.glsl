#version 440 core

#include <globals.glsl>

layout(location = 0) in vec3 v_pos;

layout(location = 0) out vec3 f_pos;

void main() {
    f_pos = v_pos;

    gl_Position = all_mat * vec4(f_pos - focus_off.xyz, 1);
}
