#version 330 core

#include <globals.glsl>
#include <sky.glsl>
#include <srgb.glsl>
#include <lod.glsl>

in vec3 f_pos;

out vec4 tgt_color;

#include <sky.glsl>

void main() {
	vec3 f_norm = lod_norm(f_pos.xy);

	vec3 f_col = lod_col(f_pos.xy);

	vec3 light, diffuse_light, ambient_light;
	get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	vec3 surf_color = illuminate(srgb_to_linear(f_col), light, diffuse_light, ambient_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x, cam_pos.xyz, f_pos, 1.0, true, clouds);
	vec3 color = mix(mix(surf_color, vec3(1), fog_level), clouds.rgb, clouds.a);

	tgt_color = vec4(color, 1.0);
}
