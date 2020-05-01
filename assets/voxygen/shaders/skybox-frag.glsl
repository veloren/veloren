#version 330 core

#include <globals.glsl>
#include <sky.glsl>

in vec3 f_pos;

layout (std140)
uniform u_locals {
	vec4 nul;
};

out vec4 tgt_color;

void main() {
	vec4 _clouds;

	vec3 cam_dir = normalize(f_pos - cam_pos.xyz);
	/* vec3 world_pos = cam_pos.xyz + cam_dir * 500000.0;
	tgt_color = vec4(get_sky_color(normalize(f_pos), time_of_day.x, cam_pos.xyz, world_pos, 1.0, true, _clouds), 1.0); */
	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);

	float dist = 100000.0;

	float refractionIndex = medium.x == 1u ? 1.0 / 1.3325 : 1.0;
	/* if (medium.x == 1u) {
		dist = UNDERWATER_MIST_DIST;
	} */
	vec3 wpos = cam_pos.xyz + /*normalize(f_pos)*/cam_dir * dist;

	tgt_color = vec4(get_sky_color(normalize(f_pos), time_of_day.x, cam_pos.xyz, wpos, 1.0, true, refractionIndex, _clouds), 1.0);
}
