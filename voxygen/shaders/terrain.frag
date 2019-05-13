#version 330 core

#include <globals.glsl>

in vec3 f_pos;
in vec3 f_norm;
in vec3 f_col;

layout (std140)
uniform u_locals {
	vec3 model_offs;
};

out vec4 tgt_color;

void main() {
	float ambient = 0.5;

	vec3 sun_dir = normalize(vec3(1.3, 1.7, 1.1));

	float sun_diffuse = dot(sun_dir, f_norm) * 0.5;

	tgt_color = vec4(f_col * (ambient + sun_diffuse), 1.0);
}
