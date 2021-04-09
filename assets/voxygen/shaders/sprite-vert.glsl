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

layout(location = 0) in vec4 inst_mat0;
layout(location = 1) in vec4 inst_mat1;
layout(location = 2) in vec4 inst_mat2;
layout(location = 3) in vec4 inst_mat3;
// TODO: is there a better way to pack the various vertex attributes?
// TODO: ori is unused
layout(location = 4) in uint inst_pos_ori;
layout(location = 5) in uint inst_vert_page; // NOTE: this could fit in less bits
// TODO: do we need this many bits for light and glow?
layout(location = 6) in float inst_light;
layout(location = 7) in float inst_glow;
layout(location = 8) in float model_wind_sway; // NOTE: this only varies per model
layout(location = 9) in float model_z_scale; // NOTE: this only varies per model

//layout(set = 0, binding = 12) uniform utexture2D t_sprite_verts;
//layout(set = 0, binding = 13) uniform sampler s_sprite_verts;
layout(set = 0, binding = 12) restrict readonly buffer sprite_verts {
    uvec2 verts[];
};

layout (std140, set = 2, binding = 0)
uniform u_terrain_locals {
    vec3 model_offs;
    float load_time;
    ivec4 atlas_offs;
};

// TODO: consider grouping into vec4's
layout(location = 0) out vec3 f_pos;
layout(location = 1) flat out vec3 f_norm;
layout(location = 2) flat out float f_select;
layout(location = 3) out vec2 f_uv_pos;
layout(location = 4) out vec2 f_inst_light;

const float SCALE = 1.0 / 11.0;
const float SCALE_FACTOR = pow(SCALE, 1.3) * 0.2;

const int EXTRA_NEG_Z = 32768;
const int VERT_EXTRA_NEG_Z = 128;
const uint VERT_PAGE_SIZE = 256;

void main() {
    // Matrix to transform this sprite instance from model space to chunk space
    mat4 inst_mat;
    inst_mat[0] = inst_mat0;
    inst_mat[1] = inst_mat1;
    inst_mat[2] = inst_mat2;
    inst_mat[3] = inst_mat3;

    // Worldpos of the chunk that this sprite is in
    vec3 chunk_offs = model_offs - focus_off.xyz;

    f_inst_light = vec2(inst_light, inst_glow);

    // Index of the vertex data in the 1D vertex texture
    // TODO: dx12 warning to switch to uint for modulus here (test if it got speedup?)
    int vertex_index = int(uint(gl_VertexIndex) % VERT_PAGE_SIZE + inst_vert_page * VERT_PAGE_SIZE);
    //const int WIDTH = 8192; // TODO: temp
    //ivec2 tex_coords = ivec2(vertex_index % WIDTH, vertex_index / WIDTH);
    //uvec2 pos_atlas_pos_norm_ao = texelFetch(usampler2D(t_sprite_verts, s_sprite_verts), tex_coords, 0).xy;
    uvec2 pos_atlas_pos_norm_ao = verts[vertex_index];
    uint v_pos_norm = pos_atlas_pos_norm_ao.x;
    uint v_atlas_pos = pos_atlas_pos_norm_ao.y;

    // Expand the model vertex position bits into float values
    vec3 v_pos = vec3(ivec3((uvec3(v_pos_norm) >> uvec3(0, 8, 16)) & uvec3(0xFFu, 0xFFu, 0x0FFFu)) - ivec3(0, 0, VERT_EXTRA_NEG_Z));

    // Transform into chunk space and scale
    f_pos = (inst_mat * vec4(v_pos, 1.0)).xyz;
    // Transform info world space
    f_pos += chunk_offs;

    // Terrain 'pop-in' effect
    f_pos.z -= 250.0 * (1.0 - min(1.0001 - 0.02 / pow(tick.x - load_time, 10.0), 1.0));

    // Wind sway effect
    f_pos += model_wind_sway * vec3(
        sin(tick.x * 1.5 + f_pos.y * 0.1) * sin(tick.x * 0.35),
        sin(tick.x * 1.5 + f_pos.x * 0.1) * sin(tick.x * 0.25),
        0.0
        // NOTE: could potentially replace `v_pos.z * model_z_scale` with a calculation using `inst_chunk_pos` from below
        //) * pow(abs(v_pos.z * model_z_scale), 1.3) * SCALE_FACTOR;
        ) * v_pos.z * model_z_scale * SCALE_FACTOR;

    // Determine normal
    // TODO: do changes here effect perf on vulkan
    // TODO: dx12 doesn't like dynamic index
    // TODO: use mix?
    // Shader@0x000001AABD89BEE0(112,43-53): error X4576: Input array signature parameter  cannot be indexed dynamically.
    //vec3 norm = (inst_mat[(v_pos_norm >> 30u) & 3u].xyz);
    uint index = v_pos_norm >> 30u & 3u;
    vec3 norm;
    if (index == 0) {
        norm = (inst_mat[0].xyz);
    } else if (index == 1) {
        norm = (inst_mat[1].xyz);
    } else {
        norm = (inst_mat[2].xyz);
    }

    f_norm = normalize(mix(-norm, norm, v_pos_norm >> 29u & 1u));

    // Expand atlas tex coords to floats
    // NOTE: Could defer to fragment shader if we are vert heavy
    f_uv_pos = vec2((uvec2(v_atlas_pos) >> uvec2(0, 16)) & uvec2(0xFFFFu, 0xFFFFu));;

    // Position of the sprite block in the chunk
    // Used solely for highlighting the selected sprite 
    vec3 inst_chunk_pos = vec3(ivec3((uvec3(inst_pos_ori) >> uvec3(0, 6, 12)) & uvec3(0x3Fu, 0x3Fu, 0xFFFFu)) - ivec3(0, 0, EXTRA_NEG_Z));
    // Select glowing
    vec3 sprite_pos = inst_chunk_pos + chunk_offs;
    f_select = (select_pos.w > 0 && select_pos.xyz == sprite_pos) ? 1.0 : 0.0;

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
}
