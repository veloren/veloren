#version 330 core

in vec2 v_pos;
in vec2 v_uv;
in vec4 v_color;
in uint v_mode;

uniform sampler2D u_tex;

out vec2 f_uv;
flat out uint f_mode;
out vec4 f_color;

void main() {
    f_uv = v_uv;
    f_color = v_color;
    gl_Position = vec4(v_pos, 0.0, 1.0);
    f_mode = v_mode;
}
