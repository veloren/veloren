#version 330 core

#include <globals.glsl>

in vec3 f_pos;
flat in uint f_pos_norm;
in vec3 f_col;
in float f_light;

layout (std140)
uniform u_locals {
	vec3 model_offs;
};

out vec4 tgt_color;

void main() {
	// Calculate normal from packed data
	vec3 f_norm;
	uint norm_axis = (f_pos_norm >> 30) & 0x3u;
	float norm_dir = float((f_pos_norm >> 29) & 0x1u) * 2.0 - 1.0;
	if (norm_axis == 0u) {
		f_norm = vec3(1.0, 0.0, 0.0) * norm_dir;
	} else if (norm_axis == 1u) {
		f_norm = vec3(0.0, 1.0, 0.0) * norm_dir;
	} else {
		f_norm = vec3(0.0, 0.0, 1.0) * norm_dir;
	}

	float glob_ambience = 0.001;

	float sun_ambience = 0.9;

	vec3 sun_dir = normalize(vec3(1.3, 1.7, 2.1));

	float sun_diffuse = dot(sun_dir, f_norm);
	float sun_light = sun_ambience + sun_diffuse;

	float static_light = glob_ambience + min(sun_light, f_light);

	vec3 light = vec3(static_light);

	tgt_color = vec4(f_col * light, 1.0);
}
