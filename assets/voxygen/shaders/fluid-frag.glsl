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
    vec3 warped_norm = normalize(f_norm + smooth_rand(f_pos * 0.35, tick.x) * 0.2);

	vec3 light = get_sun_diffuse(warped_norm, time_of_day.x) * f_light + light_at(f_pos, warped_norm);
	vec3 surf_color = f_col * light;

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
    vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x);

	vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	vec3 reflect_color = get_sky_color(reflect(cam_to_frag, warped_norm), time_of_day.x) * f_light;
	float passthrough = dot(faceforward(warped_norm, warped_norm, cam_to_frag), -cam_to_frag);

	vec4 color = mix(vec4(reflect_color, 1.0), vec4(surf_color, 0.8 / (1.0 + light * 2.0)), passthrough);

    tgt_color = mix(color, vec4(fog_color, 0.0), fog_level);
}
