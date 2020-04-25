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
	float distance = distance(vec3(cam_pos), vec3(model_mat * vec4(vec3(0), 1))) - 2;
		
	float opacity = clamp(distance / distance_divider, 0, 1);

	if(threshold_matrix[int(gl_FragCoord.x) % 4][int(gl_FragCoord.y) % 4] > opacity) {
		discard;
	}

	if(threshold_matrix[int(gl_FragCoord.x) % 4][int(gl_FragCoord.y) % 4] > shadow_dithering) {
		discard;
	}

	tgt_color = vec4(0.0,0.0,0.0, 1.0);
}