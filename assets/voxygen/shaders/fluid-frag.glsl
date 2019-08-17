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

vec3 warp_normal(vec3 norm, vec3 pos, float time) {
	return normalize(norm
		+ smooth_rand(pos * 1.0, time * 1.0) * 0.05
		+ smooth_rand(pos * 0.25, time * 0.25) * 0.1);
}

void main() {
	/*
	// Round the position to the nearest triangular grid cell
	vec3 hex_pos = f_pos * 2.0;
	hex_pos = hex_pos + vec3(hex_pos.y * 1.4 / 3.0, hex_pos.y * 0.1, 0);
	if (fract(hex_pos.x) > fract(hex_pos.y)) {
		hex_pos += vec3(1.0, 1.0, 0);
	}
	hex_pos = floor(hex_pos);
	*/

	vec3 norm = warp_normal(f_norm, f_pos, tick.x);

	vec3 light = get_sun_diffuse(norm, time_of_day.x) * f_light + light_at(f_pos, norm);
	vec3 surf_color = f_col * light;

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
    vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x);

	vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	vec3 reflect_ray_dir = reflect(cam_to_frag, norm);
	// Hack to prevent the reflection ray dipping below the horizon and creating weird blue spots in the water
	reflect_ray_dir.z = max(reflect_ray_dir.z, 0.05);

	vec3 reflect_color = get_sky_color(reflect_ray_dir, time_of_day.x) * f_light;
	// 0 = 100% reflection, 1 = translucent water
	float passthrough = pow(dot(faceforward(norm, norm, cam_to_frag), -cam_to_frag), 1.0);

	vec4 color = mix(vec4(reflect_color, 1.0), vec4(surf_color, 0.5 / (1.0 + light * 2.0)), passthrough);

    tgt_color = mix(color, vec4(fog_color, 0.0), fog_level);
}
