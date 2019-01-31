#version 330 core

in vec3 f_pos;
in vec2 f_uv;

layout (std140)
uniform u_locals {
	vec4 bounds;
};

uniform sampler2D u_tex;

out vec4 tgt_color;

void main() {
	tgt_color = texture(u_tex, f_uv);
}
