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

void main() {
	// First 3 normals are negative, next 3 are positive
	vec3 normals[6] = vec3[]( vec3(-1,0,0), vec3(0,-1,0), vec3(0,0,-1), vec3(1,0,0), vec3(0,1,0), vec3(0,0,1) );

	// TODO: last 3 bits in v_pos_norm should be a number between 0 and 5, rather than 0-2 and a direction.
	uint norm_axis = (f_pos_norm >> 30) & 0x3u;
	// Increase array access by 3 to access positive values
	uint norm_dir = ((f_pos_norm >> 29) & 0x1u) * 3u;
	// Use an array to avoid conditional branching
	vec3 f_norm = normals[norm_axis + norm_dir];

    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	vec3 surf_color = /*srgb_to_linear*/(vec3(0.4, 0.7, 2.0));

    vec3 k_a = 0.5 * surf_color;
    vec3 k_d = vec3(1.0);
    vec3 k_s = 0.5 * vec3(1.0);
    float alpha = 0.255;

    vec3 emitted_light, reflected_light;

	// float point_shadow = shadow_at(f_pos, f_norm);
	// vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	// vec3 emitted_light, reflected_light;
	// vec3 light, diffuse_light, ambient_light;
	float point_shadow = shadow_at(f_pos,f_norm);
	// vec3 surf_color = /*srgb_to_linear*/(vec3(0.4, 0.7, 2.0));
    get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, k_a * f_light * point_shadow, vec3(0.0), k_s * f_light * point_shadow, alpha, emitted_light, reflected_light);
	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 0.0);
	// diffuse_light *= f_light * point_shadow;
	// ambient_light *= f_light, point_shadow;
	// vec3 point_light = light_at(f_pos, f_norm);
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;
	// vec3 surf_color = srgb_to_linear(vec3(0.4, 0.7, 2.0)) * light * diffuse_light * ambient_light;
    lights_at(f_pos, f_norm, cam_to_frag, k_a * f_light * point_shadow, k_d * f_light * point_shadow, k_s * f_light * point_shadow, alpha, emitted_light, reflected_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
    vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x, cam_pos.xyz, f_pos, 0.25, true, clouds);

	float passthrough = pow(dot(faceforward(f_norm, f_norm, cam_to_frag), -cam_to_frag), 0.5);

    surf_color = illuminate(emitted_light, reflected_light);
	vec4 color = mix(vec4(surf_color, 1.0), vec4(surf_color, 1.0 / (1.0 + /*diffuse_light*//*(f_light * point_shadow + point_light)*/reflected_light * 0.25)), passthrough);

    tgt_color = mix(mix(color, vec4(fog_color, 0.0), fog_level), vec4(clouds.rgb, 0.0), clouds.a);
}
