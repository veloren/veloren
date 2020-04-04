#version 330 core

#include <globals.glsl>
#include <sky.glsl>
#include <lod.glsl>

in vec3 f_pos;
in vec3 f_norm;

out vec4 tgt_color;

#include <sky.glsl>

void main() {
	vec3 f_norm = lod_norm(f_pos.xy);

	vec3 f_col = lod_col(f_pos.xy);

	vec3 emitted_light, reflected_light;
    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	// vec3 light, diffuse_light, ambient_light;
    get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, f_col, 0.5 * f_col, 0.5 * vec3(1.0), 2.0, emitted_light, reflected_light);
	// vec3 light, diffuse_light, ambient_light;
	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	// vec3 surf_color = illuminate(f_col, light, diffuse_light, ambient_light);
    vec3 surf_color = illuminate(emitted_light, reflected_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);

	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag, time_of_day.x, cam_pos.xyz, f_pos, 1.0, true, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);

	float mist_factor = max(1 - (f_pos.z + (texture(t_noise, f_pos.xy * 0.0005 + time_of_day.x * 0.0003).x - 0.5) * 128.0) / 400.0, 0.0);
	//float mist_factor = f_norm.z * 2.0;
	color = mix(color, vec3(1.0) * /*diffuse_light*/reflected_light, clamp(mist_factor * 0.00005 * distance(f_pos.xy, focus_pos.xy), 0, 0.3));

	tgt_color = vec4(color, 1.0);
}
