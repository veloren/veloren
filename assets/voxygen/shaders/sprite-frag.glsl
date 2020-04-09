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
    vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    vec3 view_dir = normalize(-vec3(vert_pos4) / vert_pos4.w);
	vec3 surf_color = /*srgb_to_linear*/(f_col);

    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(0.5);
    vec3 k_s = vec3(0.5);
    float alpha = 2.0;

    vec3 emitted_light, reflected_light;

	float point_shadow = shadow_at(f_pos, f_norm);
    vec3 light_frac = light_reflection_factor(f_norm, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(1.0), 2.0);

	// vec3 light, diffuse_light, ambient_light;
    // vec3 emitted_light, reflected_light;
	// float point_shadow = shadow_at(f_pos,f_norm);
	// vec3 point_light = light_at(f_pos, f_norm);
	// vec3 surf_color = srgb_to_linear(vec3(0.2, 0.5, 1.0));
    // vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, k_a * (0.5 * f_light * point_shadow + 0.5 * light_frac), k_d * f_light * point_shadow, k_s * f_light * point_shadow, 2.0, emitted_light, reflected_light);
	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	// float point_shadow = shadow_at(f_pos, f_norm);
	// diffuse_light *= f_light * point_shadow;
	// ambient_light *= f_light * point_shadow;
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;
    lights_at(f_pos, f_norm, cam_to_frag, k_a, k_d, k_s, alpha, emitted_light, reflected_light);
	surf_color = illuminate(surf_color * emitted_light, surf_color * reflected_light);
	// vec3 surf_color = illuminate(f_col, light, diffuse_light, ambient_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag, time_of_day.x, cam_pos.xyz, f_pos, 0.5, true, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);

	tgt_color = vec4(color, 1.0 - clamp((distance(focus_pos.xy, f_pos.xy) - (RENDER_DIST - FADE_DIST)) / FADE_DIST, 0, 1));
}
