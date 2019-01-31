#version 330 core

in vec3 v_pos;
in vec2 v_uv;

layout (std140)
uniform u_locals {
	vec4 bounds;
};

uniform sampler2D u_tex;

out vec3 f_pos;
out vec2 f_uv;

void main() {
	f_uv = v_uv;
	f_pos = vec3(vec2(bounds.x, bounds.y) + v_pos.xy * vec2(bounds.z, bounds.w), 0);
	f_pos.xy = vec2(f_pos.x * 2.0 - 1.0, f_pos.y * -2.0 + 1.0);

	gl_Position = vec4(f_pos, 1);
}
