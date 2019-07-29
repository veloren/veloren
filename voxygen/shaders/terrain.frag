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

#include <sky.glsl>
#include <light.glsl>

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

	vec3 light = (get_sun_diffuse(f_norm, time_of_day.x) + light_at(f_pos, f_norm)) * f_light;
	vec3 surf_color = f_col * light;

	float fog_level = fog(f_pos.xy, focus_pos.xy);
	vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x);
	vec3 color = mix(surf_color, fog_color, fog_level);

	tgt_color = vec4(color, 1.0);
}
