#version 330 core

#include <globals.glsl>
#include <random.glsl>

in vec3 f_pos;
flat in vec3 f_norm;
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
	/*
	vec3 hex_pos = f_pos * 2.0;
	hex_pos = hex_pos + vec3(hex_pos.y * 1.4 / 3.0, hex_pos.y * 0.1, 0);
	if (fract(hex_pos.x) > fract(hex_pos.y)) {
		hex_pos += vec3(1.0, 1.0, 0);
	}
	hex_pos = f_pos;//floor(hex_pos);
	*/

    vec3 warped_norm = normalize(f_norm
		+ smooth_rand(f_pos * 1.0, tick.x * 1.0) * 0.05
		+ smooth_rand(f_pos * 0.25, tick.x * 0.25) * 0.1);

	vec3 light = get_sun_diffuse(warped_norm, time_of_day.x) * f_light + light_at(f_pos, warped_norm);
	vec3 surf_color = f_col * light;

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
    vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x);

	vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	vec3 reflect_ray_dir = reflect(cam_to_frag, warped_norm);
	reflect_ray_dir.z = max(reflect_ray_dir.z, 0.05);
	vec3 reflect_color = get_sky_color(reflect_ray_dir, time_of_day.x) * f_light;
	float passthrough = pow(dot(faceforward(warped_norm, warped_norm, cam_to_frag), -cam_to_frag), 1.0);

	vec4 color = mix(vec4(reflect_color, 1.0), vec4(surf_color, 0.5 / (1.0 + light * 2.0)), passthrough);

    tgt_color = mix(color, vec4(fog_color, 0.0), fog_level);
}
