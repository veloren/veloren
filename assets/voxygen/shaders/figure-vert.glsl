#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <lod.glsl>

in uint v_pos_norm;
in uint v_atlas_pos;

// in vec3 v_norm;
/* in uint v_col;
// out vec3 light_pos[2];
in uint v_ao_bone; */

layout (std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
    ivec4 atlas_offs;
    vec3 model_pos;
	// bit 0 - is player
	// bit 1-31 - unused
	int flags;
};

struct BoneData {
	mat4 bone_mat;
    mat4 normals_mat;
};

layout (std140)
uniform u_bones {
	// Warning: might not actually be 16 elements long. Don't index out of bounds!
	BoneData bones[16];
};

//struct ShadowLocals {
//	mat4 shadowMatrices;
//    mat4 texture_mat;
//};
//
//layout (std140)
//uniform u_light_shadows {
//    ShadowLocals shadowMats[/*MAX_LAYER_FACES*/192];
//};

out vec3 f_pos;
// flat out uint f_pos_norm;
flat out vec3 f_norm;
// float dummy;
/*centroid */out vec2 f_uv_pos;
// out vec3 f_col;
// out float f_ao;
// out float f_alt;
// out vec4 f_shadow;

// #if (SHADOW_MODE == SHADOW_MODE_MAP)
// out vec4 sun_pos;
// #endif

void main() {
	// Pre-calculate bone matrix
	/* uint bone_idx = (v_ao_bone >> 2) & 0x3Fu; */
	uint bone_idx = (v_pos_norm >> 27) & 0xFu;
    BoneData bone_data = bones[bone_idx];
    mat4 bone_mat = bone_data.bone_mat;
	mat4 combined_mat = /*model_mat * */bone_mat;

	vec3 pos = (vec3((uvec3(v_pos_norm) >> uvec3(0, 9, 18)) & uvec3(0x1FFu)) - 256.0) / 2.0;

    // vec4 bone_pos = bones[bone_idx].bone_mat * vec4(pos, 1);

	f_pos = (
        combined_mat *
        vec4(pos, 1.0)
    ).xyz + (model_pos - focus_off.xyz);

	/* f_pos.z -= 25.0 * pow(distance(focus_pos.xy, f_pos.xy) / view_distance.x, 20.0); */

    f_uv_pos = vec2((uvec2(v_atlas_pos) >> uvec2(2, 17)) & uvec2(0x7FFFu, 0x7FFFu));

	// f_col = srgb_to_linear(vec3((uvec3(v_col) >> uvec3(0, 8, 16)) & uvec3(0xFFu)) / 255.0);
	// f_col = vec3(1.0);

	// f_ao = float(v_ao_bone & 0x3u) / 4.0;
    // f_ao = 1.0;
    /* for (uint i = 0u; i < light_shadow_count.z; ++i) {
        light_pos[i] = vec3(shadowMats[i].texture_mat * vec4(f_pos, 1.0));
    } */

	// First 3 normals are negative, next 3 are positive
    // uint normal_idx = ((v_atlas_pos & 3u) << 1u) | (v_pos_norm >> 31u);
	// const vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));
    // vec3 norm = normals[normal_idx];
    uint axis_idx = v_atlas_pos & 3u;

    vec3 norm = bone_data.normals_mat[axis_idx].xyz;
    // norm = normalize(norm);
    // vec3 norm = norm_mat * vec4(uvec3(1 << axis_idx) & uvec3(0x1u, 0x3u, 0x7u), 1);

	// // Calculate normal here rather than for each pixel in the fragment shader
	// f_norm = normalize((
	// 	combined_mat *
    //     vec4(norm, 0)
	// ).xyz);
    f_norm = mix(-norm, norm, v_pos_norm >> 31u);

// #if (SHADOW_MODE == SHADOW_MODE_MAP)
//     // for (uint i = 0u; i < light_shadow_count.z; ++i) {
//     //     light_pos[i] = /*vec3(*/shadowMats[i].texture_mat * vec4(f_pos, 1.0)/*)*/;
//     // }
//     sun_pos = /*vec3(*/shadowMats[0].texture_mat * vec4(f_pos, 1.0)/*)*/;
// // #elif (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_NONE)
// //    vec4 sun_pos = vec4(0.0);
// #endif

    // f_pos_norm = v_pos_norm;

    // Also precalculate shadow texture and estimated terrain altitude.
    // f_alt = alt_at(f_pos.xy);
    // f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));

	gl_Position = all_mat/*shadowMats[0].shadowMatrices*/ * vec4(f_pos, 1);
    // gl_Position.z = -gl_Position.z / 100.0 / gl_Position.w;
	// gl_Position.z = -gl_Position.z / 100.0;
	// gl_Position.z = gl_Position.z / 100.0;
	// gl_Position.z = -gl_Position.z;
	// gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
