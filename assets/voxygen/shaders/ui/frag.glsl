#version 330 core

#include <globals.glsl>

in vec2 f_uv;
in vec4 f_color;
flat in uint f_mode;

layout (std140)
uniform u_locals {
	vec4 w_pos;
};

uniform sampler2D u_tex;

out vec4 tgt_color;

void main() {
    // Text
    if (f_mode == uint(0)) {
        tgt_color = f_color * vec4(1.0, 1.0, 1.0, texture(u_tex, f_uv).a);
    // Image
    } else if (f_mode == uint(1)) {
        tgt_color = f_color * texture(u_tex, f_uv);
    // 2D Geometry
    } else if (f_mode == uint(2)) {
        tgt_color = f_color;
    }
}
