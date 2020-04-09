#version 330 core

#include <globals.glsl>

in vec3 f_pos;
in vec3 f_col;
flat in vec3 f_norm;

layout (std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
};

struct BoneData {
	mat4 bone_mat;
};

layout (std140)
uniform u_bones {
	BoneData bones[16];
};

#include <sky.glsl>
#include <light.glsl>
#include <srgb.glsl>

out vec4 tgt_color;

void main() {
    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    vec3 view_dir = normalize(-vec3(vert_pos4) / vert_pos4.w);

	vec3 surf_color = /*srgb_to_linear*/(model_col.rgb * f_col);

    vec3 k_a = vec3(0.5);
    vec3 k_d = vec3(0.5);
    vec3 k_s = vec3(0.5);
    float alpha = 2.0;

    vec3 emitted_light, reflected_light;

	float point_shadow = shadow_at(f_pos, f_norm);
    vec3 light_frac = light_reflection_factor(f_norm, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(1.0), 2.0);
	// vec3 point_light = light_at(f_pos, f_norm);
	// vec3 light, diffuse_light, ambient_light;
    get_sun_diffuse(f_norm, time_of_day.x, view_dir, k_a * (0.5 * point_shadow + 0.5 * light_frac), k_d * point_shadow, k_s * point_shadow, alpha, emitted_light, reflected_light);

    lights_at(f_pos, f_norm, view_dir, k_a, k_d, k_s, alpha, emitted_light, reflected_light);
    // get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, surf_color * f_light * point_shadow, 0.5 * surf_color * f_light * point_shadow, 0.5 * surf_color * f_light * point_shadow, 2.0, emitted_light, reflected_light);

	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	// diffuse_light *= point_shadow;
	// ambient_light *= point_shadow;
	// vec3 point_light = light_at(f_pos, f_norm);
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;
	// vec3 surf_color = illuminate(srgb_to_linear(model_col.rgb * f_col), light, diffuse_light, ambient_light);
	surf_color = illuminate(surf_color * emitted_light, surf_color * reflected_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag, time_of_day.x, cam_pos.xyz, f_pos, 0.5, true, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);

	tgt_color = vec4(color, 1.0);
}
