#version 330 core

#include <globals.glsl>

in vec3 f_pos;
in vec3 f_col;
in float f_ao;
flat in vec3 f_norm;

layout (std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
	// bit 0 - is player
	// bit 1-31 - unused
	int flags;
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
	vec3 light, diffuse_light, ambient_light;
	get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	float point_shadow = shadow_at(f_pos, f_norm);
	diffuse_light *= point_shadow;
	ambient_light *= point_shadow;
	vec3 point_light = light_at(f_pos, f_norm);
	light += point_light;
	diffuse_light += point_light;

	float ao = pow(f_ao, 0.5) * 0.85 + 0.15;

	ambient_light *= ao;
	diffuse_light *= ao;

	vec3 surf_color = illuminate(srgb_to_linear(model_col.rgb * f_col), light, diffuse_light, ambient_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x, cam_pos.xyz, f_pos, 0.5, true, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);

	if ((flags & 1) == 1 && int(cam_mode) == 1) {
		float distance = distance(vec3(cam_pos), focus_pos.xyz) - 2;

		float opacity = clamp(distance / distance_divider, 0, 1);

		if(threshold_matrix[int(gl_FragCoord.x) % 4][int(gl_FragCoord.y) % 4] > opacity) {
			discard;
		}
	}

	tgt_color = vec4(color, 1.0);
}
