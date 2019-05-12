#version 330 core

#include <globals.glsl>

in vec2 v_pos;

layout (std140)
uniform u_locals {
	vec4 nul;
};

out vec2 f_pos;

void main() {
	f_pos = v_pos;

	gl_Position = vec4(v_pos, 0.0, 1.0);
}
