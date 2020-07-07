#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <srgb.glsl>
#include <sky.glsl>

in vec3 v_pos;
in uint v_atlas_pos;
// in uint v_col;
in uint v_norm_ao;
in uint inst_pos_ori;
in vec4 inst_mat0;
in vec4 inst_mat1;
in vec4 inst_mat2;
in vec4 inst_mat3;
// in vec3 inst_col;
in float inst_wind_sway;

struct SpriteLocals {
    mat4 mat;
    vec4 wind_sway;
    vec4 offs;
};

layout (std140)
uniform u_locals {
    mat4 mat;
    vec4 wind_sway;
    vec4 offs;
    // SpriteLocals sprites[8];
};

// struct Instance {
//     mat4 inst_mat;
//     vec3 inst_col;
//     float inst_wind_sway;
// };
//
// layout (std140)
// uniform u_ibuf {
//     Instance sprite_instances[/*MAX_LAYER_FACES*/512];
// };

//struct ShadowLocals {
//    mat4 shadowMatrices;
//    mat4 texture_mat;
//};
//
//layout (std140)
//uniform u_light_shadows {
//    ShadowLocals shadowMats[/*MAX_LAYER_FACES*/192];
//};

layout (std140)
uniform u_terrain_locals {
    vec3 model_offs;
    float load_time;
    ivec4 atlas_offs;
};

out vec3 f_pos;
flat out vec3 f_norm;
flat out float f_light;
// flat out vec3 f_pos_norm;
// out vec3 f_col;
// out float f_ao;
out vec2 f_uv_pos;
// flat out uint f_atlas_pos;
// out vec3 light_pos[2];
// out float f_light;

const float SCALE = 1.0 / 11.0;
const float SCALE_FACTOR = pow(SCALE, 1.3) * 0.2;

const int EXTRA_NEG_Z = 32768;

void main() {
    // vec3 inst_chunk_pos = vec3(ivec3((uvec3(inst_pos_ori) >> uvec3(0, 6, 12)) & uvec3(0x3Fu, 0x3Fu, 0xFFFFu)) - ivec3(0, 0, EXTRA_NEG_Z));
    // uint inst_ori = (inst_pos_ori >> 29) & 0x7u;
    // SpriteLocals locals = sprites[inst_ori];
    // SpriteLocals locals = sprites;
    // mat4 inst_mat = locals.mat;
    // float inst_wind_sway = locals.wind_sway.w;

    // mat4 inst_mat = mat4(vec4(1, 0, 0, 0), vec4(0, 1, 0, 0), vec4(0, 0, 1, 0), vec4(5.5, 5.5, 0, 1));
    // float inst_wind_sway = 0.0;
    mat4 inst_mat;
    inst_mat[0] = inst_mat0;
    inst_mat[1] = inst_mat1;
    inst_mat[2] = inst_mat2;
    inst_mat[3] = inst_mat3;
    /* Instance instances = sprite_instances[gl_InstanceID & 1023];
    mat4 inst_mat = instances.inst_mat;
    vec3 inst_col = instances.inst_col;
    float inst_wind_sway  = instances.inst_wind_sway; */
    vec3 inst_offs = model_offs - focus_off.xyz;
    // mat3 inst_mat;
    // inst_mat[0] = inst_mat0.xyz;
    // inst_mat[1] = inst_mat1.xyz;
    // inst_mat[2] = inst_mat2.xyz;
    // /* Instance instances = sprite_instances[gl_InstanceID & 1023];
    // mat4 inst_mat = instances.inst_mat;
    // vec3 inst_col = instances.inst_col;
    // float inst_wind_sway = instances.inst_wind_sway; */
    // float inst_wind_sway = wind_sway.w;
    // vec3 inst_offs = model_offs - focus_off.xyz;

    // vec3 sprite_pos = floor(inst_mat3.xyz * SCALE) + inst_offs;

    // f_pos_norm = v_pos;

    // vec3 sprite_pos = (inst_mat * vec4(0, 0, 0, 1)).xyz;
    // vec3 sprite_pos = floor((inst_mat * vec4(0, 0, 0, 1)).xyz * SCALE/* - vec3(0.5, 0.5, 0.0)*/) + inst_offs;
    // vec3 sprite_pos = /*round*/floor(((inst_mat * vec4(0, 0, 0, 1)).xyz - /* wind_sway.xyz * */offs.xyz) * SCALE/* - vec3(0.5, 0.5, 0.0)*/) - inst_offs;
    // vec3 sprite_pos = /*round*/floor(((inst_mat * vec4(-offs.xyz, 1)).xyz) * SCALE/* - vec3(0.5, 0.5, 0.0)*/) + inst_offs;

    // vec3 v_pos = vec3(gl_VertexID * 32, gl_VertexID % 32, 1.0);
    // f_pos = v_pos + (model_offs - focus_off.xyz);

    // vec3 v_pos = /*inst_mat*//*locals.*/wind_sway.xyz * v_pos;
    vec3 v_pos_ = /*inst_mat*//*locals.*//*sprites[0].*/wind_sway.xyz * v_pos;
    // vec3 v_pos = (/*inst_mat*/locals.mat * vec4(v_pos, 1)).xyz + vec3(0.5, 0.5, 0.0);
    // f_pos = v_pos * SCALE + (inst_chunk_pos + model_offs - focus_off.xyz);

    // vec3 v_pos_ = (inst_mat * vec4(v_pos/* * SCALE*/, 1)).xyz;
    // vec3 v_pos = (inst_mat * vec4(v_pos, 1)).xyz;
    // f_pos = v_pos + (model_offs - focus_off.xyz);

    f_pos = (inst_mat * vec4(v_pos_, 1.0)).xyz * SCALE + inst_offs;
    // f_pos = (inst_mat * v_pos_) * SCALE + sprite_pos;

    // f_pos = (inst_mat * vec4(v_pos * SCALE, 1)).xyz + (model_offs - focus_off.xyz);
    // f_pos = v_pos_ + (inst_chunk_pos + model_offs - focus_off.xyz + vec3(0.5, 0.5, 0.0));
    // f_pos.z -= min(32.0, 25.0 * pow(distance(focus_pos.xy, f_pos.xy) / view_distance.x, 20.0));

    // Wind waving
    /* const float x_scale = sin(tick.x * 1.5 + f_pos.x * 0.1);
    const float y_scale = sin(tick.x * 1.5 + f_pos.y * 0.1);
    const float z_scale = pow(abs(v_pos_.z), 1.3) * SCALE_FACTOR;
    const float xy_bias = sin(tick.x * 0.25);
    const float z_bias = xy_bias * t_scale;
    mat3 shear = mat4(
        vec3(x_scale , 0.0, 0.0, 0.0),
        vec3(0.0, y_scale, 0.0, 0.0),
        vec3(0.0, 0.0, z_bias, 0.0),
        vec3(0.0, 0.0, (1.0 / z_bias), 0.0)
    ); */
    // const float x_scale = sin(tick.x * 1.5 + f_pos.x * 0.1);
    // const float y_scale = sin(tick.x * 1.5 + f_pos.y * 0.1);
    // const float z_scale = pow(abs(v_pos_.z), 1.3) * SCALE_FACTOR;
    // const float xy_bias = sin(tick.x * 0.25);
    // const float z_bias = xy_bias * t_scale;
    // vec3 rotate = inst_wind_sway * vec3(
    // sin(tick.x * 1.5 + f_pos.y * 0.1) * sin(tick.x * 0.35),
    // sin(tick.x * 1.5 + f_pos.x * 0.1) * sin(tick.x * 0.25),
    // 0.0
    // ) * pow(abs(v_pos_.z/* + sprites[0].offs.z*/)/* * SCALE*/, 1.3) * /*0.2;*/SCALE_FACTOR;
    //
    // mat3 shear = mat4(
    //     vec3(x_scale * , 0.0, 0.0, 0.0),
    //     vec3(0.0, y_scale, 0.0, 0.0),
    //     vec3(0.0, 0.0, z_bias, 0.0),
    //     vec3(0.0, 0.0, (1.0 / z_bias), 0.0)
    // );
    /*if (wind_sway.w >= 0.4) */{
        f_pos += /*inst_wind_sway*/wind_sway.w * vec3(
            sin(tick.x * 1.5 + f_pos.y * 0.1) * sin(tick.x * 0.35),
            sin(tick.x * 1.5 + f_pos.x * 0.1) * sin(tick.x * 0.25),
            0.0
            ) * pow(abs(v_pos_.z/* + sprites[0].offs.z*/)/* * SCALE*/, 1.3) * /*0.2;*/SCALE_FACTOR;
    }

    // First 3 normals are negative, next 3 are positive
    // vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));
    // uint norm_idx = (v_norm_ao >> 0) & 0x7u;
    // f_norm = (inst_mat * vec4(normals[], 0)).xyz;

    // TODO: Consider adding a second, already-normalized (i.e. unscaled) matrix.
    // vec3 norm = /*normalize*/(inst_mat/*locals.mat*/[(v_norm_ao >> 1u) & 3u].xyz);
    // vec3 norm = /*normalize*/(inst_mat/*locals.mat*/[(v_norm_ao >> 1u) & 3u]);

    // vec3 norm = bone_data.normals_mat[axis_idx].xyz;
    // norm = normalize(norm);
    // norm = norm / SCALE_FACTOR / locals.wind_sway.xyz;
    // norm = norm / (norm.x + norm.y + norm.z);
    // vec3 norm = norm_mat * vec4(uvec3(1 << axis_idx) & uvec3(0x1u, 0x3u, 0x7u), 1);

    // // Calculate normal here rather than for each pixel in the fragment shader
    // f_norm = normalize((
    //     combined_mat *
    //     vec4(norm, 0)
    // ).xyz);

    vec3 norm = /*normalize*/(inst_mat/*locals.mat*/[(v_norm_ao >> 1u) & 3u].xyz);
    f_norm = mix(-norm, norm, v_norm_ao & 1u);

    /* vec3 col = vec3((uvec3(v_col) >> uvec3(0, 8, 16)) & uvec3(0xFFu)) / 255.0;
    f_col = srgb_to_linear(col) * srgb_to_linear(inst_col);
    f_ao = float((v_norm_ao >> 3) & 0x3u) / 4.0; */
    f_uv_pos = vec2((uvec2(v_atlas_pos) >> uvec2(0, 16)) & uvec2(0xFFFFu, 0xFFFFu));/* + 0.5*/;
    // f_atlas_pos = v_atlas_pos;
    /* for (uint i = 0u; i < light_shadow_count.z; ++i) {
        light_pos[i] = vec3(shadowMats[i].texture_mat * vec4(f_pos, 1.0));
    } */

    // // Select glowing
    // if (select_pos.w > 0 && select_pos.xyz == floor(sprite_pos)) {
    //     f_col *= 4.0;
    // }
    // f_light = 1.0;
    // if (select_pos.w > 0) */{
        vec3 sprite_pos = /*round*/floor(((inst_mat * vec4(-offs.xyz, 1)).xyz) * SCALE/* - vec3(0.5, 0.5, 0.0)*/) + inst_offs;
        f_light = (select_pos.w > 0 && select_pos.xyz == sprite_pos/* - vec3(0.5, 0.5, 0.0) * SCALE*/) ? 1.0 / PERSISTENT_AMBIANCE : 1.0;
    // }

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
    // gl_Position.z = -gl_Position.z;
    // gl_Position.z = -gl_Position.z / gl_Position.w;
    // gl_Position.z = -gl_Position.z / 100.0;
    // gl_Position.z = -gl_Position.z / 100.0;
    // gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
