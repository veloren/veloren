#version 440 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <srgb.glsl>
#include <sky.glsl>
#include <light.glsl>

layout(location = 0) in vec4 inst_mat0;
layout(location = 1) in vec4 inst_mat1;
layout(location = 2) in vec4 inst_mat2;
layout(location = 3) in vec4 inst_mat3;
// TODO: is there a better way to pack the various vertex attributes?
// TODO: ori is unused
layout(location = 4) in uint inst_pos_ori_door;
layout(location = 5) in uint inst_vert_page; // NOTE: this could fit in less bits
// TODO: do we need this many bits for light and glow?
layout(location = 6) in float inst_light;
layout(location = 7) in float inst_glow;
layout(location = 8) in float model_wind_sway; // NOTE: this only varies per model
layout(location = 9) in float model_z_scale; // NOTE: this only varies per model

layout(set = 0, binding = 15) restrict readonly buffer sprite_verts {
    uvec2 verts[];
};

layout (std140, set = 3, binding = 0)
uniform u_terrain_locals {
    mat4 model_mat;
    ivec4 atlas_offs;
    float load_time;
};

// TODO: consider grouping into vec4's
layout(location = 0) out vec3 f_pos;
layout(location = 1) flat out vec3 f_norm;
layout(location = 2) flat out float f_select;
layout(location = 3) out vec2 f_uv_pos;
layout(location = 4) out vec2 f_inst_light;

const float SCALE = 1.0 / 11.0;
const float SCALE_FACTOR = pow(SCALE, 1.3) * 0.2;

const float EXTRA_NEG_Z = 32768.0;
const float VERT_EXTRA_NEG_XY = 128.0;
const float VERT_EXTRA_NEG_Z = 128.0;
const uint VERT_PAGE_SIZE = 256;
const uint VERT_PAGE_SIZE_BITS = VERT_PAGE_SIZE - 1;

// vec4(vec3(position), distance)
vec4 nearest_entity(in vec3 sprite_pos, const float entity_radius_factor) {
    vec4 closest = vec4(vec3(0), 65536);

    for (uint i = 0u; i < light_shadow_count.y; i ++) {
        // Only access the array once
        Shadow S = shadows[i];
        vec3 shadow_pos = S.shadow_pos_radius.xyz - focus_off.xyz;
        float dist_sq = dot(sprite_pos - shadow_pos, sprite_pos - shadow_pos)
            - S.shadow_pos_radius.w * S.shadow_pos_radius.w * entity_radius_factor;
        if (dist_sq < closest.w) {
            closest = vec4(shadow_pos, dist_sq);
        }
    }
    closest.w = sqrt(max(closest.w, 0));
    return closest;
}

void main() {
    // Matrix to transform this sprite instance from model space to chunk space
    mat4 inst_mat;
    inst_mat[0] = inst_mat0;
    inst_mat[1] = inst_mat1;
    inst_mat[2] = inst_mat2;
    inst_mat[3] = inst_mat3;// + vec4(-14.5, -16.5, 0.0, 0.0);

    inst_mat = model_mat * inst_mat;

    // Worldpos of the chunk that this sprite is in
    vec3 chunk_offs = -focus_off.xyz;

    f_inst_light = vec2(inst_light, inst_glow);

    // Index of the vertex data in the 1D vertex texture
    int vertex_index = int((uint(gl_VertexIndex) & VERT_PAGE_SIZE_BITS) + inst_vert_page * VERT_PAGE_SIZE);
    uvec2 pos_atlas_pos_norm_ao = verts[vertex_index];
    uint v_pos_norm = pos_atlas_pos_norm_ao.x;
    uint v_atlas_pos = pos_atlas_pos_norm_ao.y;

    // Expand the model vertex position bits into float values
    // TODO: Use this instead, see [https://gitlab.com/veloren/veloren/-/merge_requests/3091]
    //vec3 v_pos = vec3(ivec3((uvec3(v_pos_norm) >> uvec3(0, 8, 16)) & uvec3(0xFFu, 0xFFu, 0x0FFFu)) - ivec3(VERT_EXTRA_NEG_XY, VERT_EXTRA_NEG_XY, VERT_EXTRA_NEG_Z));
    vec3 v_pos = vec3(
        float(v_pos_norm & 0xFFu) - VERT_EXTRA_NEG_XY,
        float((v_pos_norm >> 8) & 0xFFu) - VERT_EXTRA_NEG_XY,
        float((v_pos_norm >> 16) & 0x0FFFu) - VERT_EXTRA_NEG_Z
    );

    // Position of the sprite block in the chunk
    // Used for highlighting the selected sprite, and for opening doors
    vec3 sprite_pos = inst_mat[3].xyz + chunk_offs;

    #ifndef EXPERIMENTAL_BAREMINIMUM
        if((inst_pos_ori_door & (1 << 28)) != 0) {
            const float MIN_OPEN_DIST = 0.2;
            const float MAX_OPEN_DIST = 1.5;
            float min_entity_dist = nearest_entity(sprite_pos, 1.0).w;

            if (min_entity_dist < MAX_OPEN_DIST) {
                float sprite_ori = (inst_pos_ori_door >> 29) & 0x7u;
                float flip = sprite_ori <= 3 ? 1.0 : -1.0;
                float theta = mix(PI/2.0, 0, pow(max(0.0, min_entity_dist - MIN_OPEN_DIST) / (MAX_OPEN_DIST - MIN_OPEN_DIST), 1.0));
                float costheta = cos(flip * theta);
                float sintheta = sin(flip * theta);
                mat3 rot_z = mat3(
                    vec3(costheta, -sintheta, 0),
                    vec3(sintheta, costheta, 0),
                    vec3(0, 0, 1)
                );

                vec3 delta = vec3(5.5, 0, 0);
                v_pos = (rot_z * (v_pos + delta)) - delta;
            }
        }
    #endif

    // Transform into chunk space and scale
    f_pos = (inst_mat * vec4(v_pos, 1.0)).xyz;
    // Transform info world space
    f_pos += chunk_offs;

    #ifndef EXPERIMENTAL_BAREMINIMUM
        #ifdef EXPERIMENTAL_TERRAINPOP
            // Terrain 'pop-in' effect
            f_pos.z -= 250.0 * (1.0 - min(1.0001 - 0.02 / pow(time_since(load_time), 10.0), 1.0));
        #endif
    #endif

    #ifdef EXPERIMENTAL_CURVEDWORLD
        f_pos.z -= pow(distance(f_pos.xy + focus_off.xy, focus_pos.xy + focus_off.xy) * 0.05, 2);
    #endif

    #ifndef EXPERIMENTAL_BAREMINIMUM
        // Wind sway effect
        f_pos.xy += (wind_vel * 0.35 + vec2(
            wind_wave(f_pos.y * 0.1, 0.9, wind_vel.x, wind_vel.y),
            wind_wave(f_pos.x * 0.1, 1.1, wind_vel.y, wind_vel.x)
        ))
            * model_wind_sway
            //* mix(10.0, abs(v_pos.z), 1.0 / (1.0 + abs(v_pos.z) * 0.1))
            * abs(v_pos.z)
            * model_z_scale
            * SCALE_FACTOR;

        if (model_wind_sway > 0.0) {
            vec2 center = sprite_pos.xy + 0.5;
            vec4 min_entity = nearest_entity(vec3(center, sprite_pos.z), 0.0);

            const float PUSH_FACTOR = 5;

            float push_dist = max(1.0 - min_entity.w, 0.0);

            f_pos.xy += normalize(center - min_entity.xy) * v_pos.z * model_z_scale * SCALE_FACTOR * PUSH_FACTOR * push_dist;
        }
    #endif

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

    // Select glowing
    f_select = (select_pos.w > 0 && select_pos.xyz == sprite_pos) ? 1.0 : 0.0;

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
}
