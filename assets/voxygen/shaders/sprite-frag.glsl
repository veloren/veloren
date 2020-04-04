#version 330 core

#include <globals.glsl>

in vec3 f_pos;
flat in vec3 f_norm;
in vec3 f_col;
in float f_light;

out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>

const float RENDER_DIST = 112.0;
const float FADE_DIST = 32.0;

void main() {
    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	vec3 surf_color = /*srgb_to_linear*/(f_col);

    vec3 k_a = surf_color;
    vec3 k_d = 0.5 * surf_color;
    vec3 k_s = 0.5 * surf_color;
    float alpha = 2.0;

    vec3 emitted_light, reflected_light;

	float point_shadow = shadow_at(f_pos, f_norm);

	// vec3 light, diffuse_light, ambient_light;
    // vec3 emitted_light, reflected_light;
	// float point_shadow = shadow_at(f_pos,f_norm);
	// vec3 point_light = light_at(f_pos, f_norm);
	// vec3 surf_color = srgb_to_linear(vec3(0.2, 0.5, 1.0));
    // vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, k_a * f_light * point_shadow, k_d * f_light * point_shadow, k_s * f_light * point_shadow, 2.0, emitted_light, reflected_light);
	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	// float point_shadow = shadow_at(f_pos, f_norm);
	// diffuse_light *= f_light * point_shadow;
	// ambient_light *= f_light * point_shadow;
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;
    lights_at(f_pos, f_norm, cam_to_frag, k_a * f_light * point_shadow, k_d * f_light * point_shadow, k_s * f_light * point_shadow, alpha, emitted_light, reflected_light);
	surf_color = illuminate(emitted_light, reflected_light);
	// vec3 surf_color = illuminate(f_col, light, diffuse_light, ambient_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag, time_of_day.x, cam_pos.xyz, f_pos, 0.5, true, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);

	tgt_color = vec4(color, 1.0 - clamp((distance(focus_pos.xy, f_pos.xy) - (RENDER_DIST - FADE_DIST)) / FADE_DIST, 0, 1));
}
