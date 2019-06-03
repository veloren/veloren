#version 330 core

#include <globals.glsl>

in vec3 f_pos;
in vec3 f_norm;
in vec3 f_col;
in float f_light;

layout (std140)
uniform u_locals {
	vec3 model_offs;
};

out vec4 tgt_color;

void main() {
	float glob_ambience = 0.001;

	float sun_ambience = 0.9;

	vec3 sun_dir = normalize(vec3(1.3, 1.7, 2.1));

	float sun_diffuse = dot(sun_dir, f_norm);
	float sun_light = sun_ambience + sun_diffuse;

	float static_light = glob_ambience + min(sun_light, f_light);

	vec3 light = static_light;

	tgt_color = vec4(f_col * light, 1.0);
}
