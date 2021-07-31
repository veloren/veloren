#version 420 core

#include <globals.glsl>

layout(location = 0) in vec2 f_uv;
layout(location = 1) in vec4 f_color;
layout(location = 2) flat in uint f_mode;

layout (std140, set = 1, binding = 0)
uniform u_locals {
    vec4 w_pos;
};

layout(set = 2, binding = 0)
uniform texture2D t_tex;
layout(set = 2, binding = 1)
uniform sampler s_tex;

layout(location = 0) out vec4 tgt_color;

void main() {
    // Text
    if (f_mode == uint(0)) {
        tgt_color = f_color * vec4(1.0, 1.0, 1.0, textureLod(sampler2D(t_tex, s_tex), f_uv, 0).a);
    // Image
    // HACK: bit 0 is set for both ordinary and north-facing images.
    } else if ((f_mode & uint(1)) == uint(1)) {
        tgt_color = f_color * textureLod(sampler2D(t_tex, s_tex), f_uv, 0);
    // 2D Geometry
    } else if (f_mode == uint(2)) {
        tgt_color = f_color;
    }
}
