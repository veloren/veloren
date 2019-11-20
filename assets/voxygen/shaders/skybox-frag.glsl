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
	tgt_color = vec4(get_sky_color(normalize(f_pos), time_of_day.x, cam_pos.xyz, vec3(-100000), 1.0, true, _clouds), 1.0);
}
