#version 330 core

#include <globals.glsl>

in vec3 f_pos;
in vec3 f_col;
flat in vec3 f_norm;

layout (std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
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
	tgt_color = vec4(0.0,0.0,0.0, 1.0);
}