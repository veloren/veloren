#version 330 core

#include <globals.glsl>
#include <sky.glsl>

in vec3 f_pos;
in vec3 f_norm;
in vec3 f_col;
flat in uint f_bone_idx;

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

out vec4 tgt_color;

void main() {
	vec3 world_norm = (
		model_mat *
		bones[f_bone_idx].bone_mat *
		vec4(f_norm, 0.0)
	).xyz;

	float ambient = 0.5;

	vec3 sun_dir = normalize(vec3(1.3, 1.7, 1.1));

	float sun_diffuse = dot(sun_dir, world_norm) * 0.5;

	vec3 surf_color = model_col.rgb * f_col * (ambient + sun_diffuse);

	float fog_level = fog(f_pos.xy, cam_pos.xy);
	vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x);
	vec3 color = mix(surf_color, fog_color, fog_level);

	tgt_color = vec4(color, 1.0);
}
