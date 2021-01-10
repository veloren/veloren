#version 420 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <srgb.glsl>
#include <sky.glsl>

layout(location = 0) in vec3 v_pos;
layout(location = 1) in uint v_atlas_pos;
layout(location = 2) in uint v_norm_ao;
layout(location = 3) in uint inst_pos_ori;
layout(location = 4) in vec4 inst_mat0;
layout(location = 5) in vec4 inst_mat1;
layout(location = 6) in vec4 inst_mat2;
layout(location = 7) in vec4 inst_mat3;
layout(location = 8) in vec4 inst_light;
layout(location = 9) in float inst_wind_sway;

struct SpriteLocals {
    mat4 mat;
    vec4 wind_sway;
    vec4 offs;
};

layout(std140, set = 3, binding = 0)
uniform u_locals {
    mat4 mat;
    vec4 wind_sway;
    vec4 offs;
};

layout (std140, set = 2, binding = 0)
uniform u_terrain_locals {
    vec3 model_offs;
    float load_time;
    ivec4 atlas_offs;
};

layout(location = 0) out vec3 f_pos;
layout(location = 1) flat out vec3 f_norm;
layout(location = 2) flat out float f_select;
layout(location = 3) out vec2 f_uv_pos;
layout(location = 4) out vec2 f_inst_light;

const float SCALE = 1.0 / 11.0;
const float SCALE_FACTOR = pow(SCALE, 1.3) * 0.2;

const int EXTRA_NEG_Z = 32768;
//const int VERT_EXTRA_NEG_Z = 128;
//const int VERT_PAGE_SIZE = 256;

void main() {
    mat4 inst_mat;
    inst_mat[0] = inst_mat0;
    inst_mat[1] = inst_mat1;
    inst_mat[2] = inst_mat2;
    inst_mat[3] = inst_mat3;
    vec3 inst_offs = model_offs - focus_off.xyz;

    f_inst_light = inst_light.xy;

    vec3 v_pos_ = wind_sway.xyz * v_pos;

    f_pos = (inst_mat * vec4(v_pos_, 1.0)).xyz * SCALE + inst_offs;

    // Terrain 'pop-in' effect
    f_pos.z -= 250.0 * (1.0 - min(1.0001 - 0.02 / pow(tick.x - load_time, 10.0), 1.0));

    f_pos += wind_sway.w * vec3(
        sin(tick.x * 1.5 + f_pos.y * 0.1) * sin(tick.x * 0.35),
        sin(tick.x * 1.5 + f_pos.x * 0.1) * sin(tick.x * 0.25),
        0.0
        //) * pow(abs(v_pos_.z), 1.3) * SCALE_FACTOR;
        ) * v_pos_.z * SCALE_FACTOR;

    vec3 norm = (inst_mat[(v_norm_ao >> 1u) & 3u].xyz);
    f_norm = mix(-norm, norm, v_norm_ao & 1u);

    f_uv_pos = vec2((uvec2(v_atlas_pos) >> uvec2(0, 16)) & uvec2(0xFFFFu, 0xFFFFu));/* + 0.5*/;

    // Select glowing
    vec3 sprite_pos = floor(((inst_mat * vec4(-offs.xyz, 1)).xyz) * SCALE) + inst_offs;
    f_light = (select_pos.w > 0 && select_pos.xyz == sprite_pos) ? 1.0 : 0.0;

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
}
