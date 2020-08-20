#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>

in vec3 f_pos;
in vec3 f_col;
flat in vec3 f_norm;
in float f_ao;
// in float f_alt;
// in vec4 f_shadow;

layout (std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
    ivec4 atlas_offs;
    vec3 model_pos;
	int flags;
};

struct BoneData {
	mat4 bone_mat;
    mat4 normals_mat;
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
	// float distance = distance(vec3(cam_pos), focus_pos.xyz) - 2;

	// float opacity = clamp(distance / distance_divider, 0, 1);

	// if(threshold_matrix[int(gl_FragCoord.x) % 4][int(gl_FragCoord.y) % 4] > opacity) {
	// 	discard;
	// }

	// if(threshold_matrix[int(gl_FragCoord.x) % 4][int(gl_FragCoord.y) % 4] > shadow_dithering) {
	// 	discard;
	// }

	tgt_color = vec4(0.0,0.0,0.0, 1.0);
}
