#version 330 core

#include <globals.glsl>

in vec2 v_pos;
in vec2 v_uv;
in vec4 v_color;
in uint v_mode;

layout (std140)
uniform u_locals {
	vec4 w_pos;
};

uniform sampler2D u_tex;

out vec2 f_uv;
flat out uint f_mode;
out vec4 f_color;

void main() {
    f_uv = v_uv;
    f_color = v_color;

    if (w_pos.w == 1.0) {
        // In-game element
        gl_Position = proj_mat * (view_mat * w_pos + vec4(v_pos, 0.0, 0.0));
    } else {
        // Interface element
        gl_Position = vec4(v_pos, 0.0, 1.0);
    }
    f_mode = v_mode;
}
