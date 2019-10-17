#version 330 core

#include <globals.glsl>
#include <random.glsl>

in vec3 f_pos;
flat in uint f_pos_norm;
in vec3 f_col;
in float f_light;

layout (std140)
uniform u_locals {
    vec3 model_offs;
	float load_time;
};

uniform sampler2D t_waves;

out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>

vec3 warp_normal(vec3 norm, vec3 pos, float time) {
	return normalize(norm
		+ smooth_rand(pos * 1.0, time * 1.0) * 0.05
		+ smooth_rand(pos * 0.25, time * 0.25) * 0.1);
}

void main() {
	// First 3 normals are negative, next 3 are positive
	vec3 normals[6] = vec3[]( vec3(-1,0,0), vec3(0,-1,0), vec3(0,0,-1), vec3(1,0,0), vec3(0,1,0), vec3(0,0,1) );

	// TODO: last 3 bits in v_pos_norm should be a number between 0 and 5, rather than 0-2 and a direction.
	uint norm_axis = (f_pos_norm >> 30) & 0x3u;
	// Increase array access by 3 to access positive values
	uint norm_dir = ((f_pos_norm >> 29) & 0x1u) * 3u;
	// Use an array to avoid conditional branching
	vec3 f_norm = normals[norm_axis + norm_dir];

	/*
	// Round the position to the nearest triangular grid cell
	vec3 hex_pos = f_pos * 2.0;
	hex_pos = hex_pos + vec3(hex_pos.y * 1.4 / 3.0, hex_pos.y * 0.1, 0);
	if (fract(hex_pos.x) > fract(hex_pos.y)) {
		hex_pos += vec3(1.0, 1.0, 0);
	}
	hex_pos = floor(hex_pos);
	*/

	vec3 b_norm;
	if (f_norm.z > 0.0) {
		b_norm = vec3(1, 0, 0);
	} else if (f_norm.x > 0.0) {
		b_norm = vec3(0, 1, 0);
	} else {
		b_norm = vec3(0, 0, 1);
	}
	vec3 c_norm = cross(f_norm, b_norm);

	vec3 nmap = normalize(
		(srgb_to_linear(texture(t_waves, fract(f_pos.xy * 0.3 + tick.x * 0.04)).rgb) - 0.0) * 0.05
		+ (srgb_to_linear(texture(t_waves, fract(f_pos.xy * 0.1 - tick.x * 0.08)).rgb) - 0.0) * 0.1
		+ (srgb_to_linear(texture(t_waves, fract(-f_pos.yx * 0.06 - tick.x * 0.1)).rgb) - 0.0) * 0.1
		+ (srgb_to_linear(texture(t_waves, fract(-f_pos.yx * 0.03 - tick.x * 0.01)).rgb) - 0.0) * 0.2
		+ vec3(0, 0, 0.0)
	);

	vec3 norm = f_norm * nmap.z + b_norm * nmap.x + c_norm * nmap.y;

	vec3 light, diffuse_light, ambient_light;
	get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 0.0);
	float point_shadow = shadow_at(f_pos, f_norm);
	diffuse_light *= f_light * point_shadow;
	ambient_light *= f_light, point_shadow;
	vec3 point_light = light_at(f_pos, f_norm);
	light += point_light;
	diffuse_light += point_light;
	vec3 surf_color = illuminate(srgb_to_linear(f_col), light, diffuse_light, ambient_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
    vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x, true);

	vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	vec3 reflect_ray_dir = reflect(cam_to_frag, norm);
	// Hack to prevent the reflection ray dipping below the horizon and creating weird blue spots in the water
	reflect_ray_dir.z = max(reflect_ray_dir.z, 0.05);

	vec3 reflect_color = get_sky_color(reflect_ray_dir, time_of_day.x, false) * f_light;
	//reflect_color = vec3(reflect_color.r + reflect_color.g + reflect_color.b) / 3.0;
	// 0 = 100% reflection, 1 = translucent water
	float passthrough = pow(dot(faceforward(f_norm, f_norm, cam_to_frag), -cam_to_frag), 0.3);

	vec4 color = mix(vec4(reflect_color * 2.0, 1.0), vec4(surf_color, 4.0 / (1.0 + diffuse_light * 2.0)), passthrough);

    tgt_color = mix(color, vec4(fog_color, 0.0), fog_level);
}
