#version 420 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <srgb.glsl>
#include <random.glsl>
#include <lod.glsl>

layout(location = 0) in vec3 v_pos;
layout(location = 1) in vec3 v_norm;
layout(location = 2) in vec3 v_col;
layout(location = 3) in vec3 inst_pos;
layout(location = 4) in uint inst_flags;

const uint FLAG_SNOW_COVERED = 1;

layout(location = 0) out vec3 f_pos;
layout(location = 1) out vec3 f_norm;
layout(location = 2) out vec4 f_col;
layout(location = 3) out vec3 model_pos;
layout(location = 4) out float snow_cover;

void main() {
    vec3 tree_pos = inst_pos - focus_off.xyz;
    f_pos = tree_pos + v_pos;
    model_pos = v_pos;

    float pull_down = 1.0 / pow(distance(focus_pos.xy, tree_pos.xy) / (view_distance.x * 0.95), 150.0);
    #ifndef EXPERIMENTAL_NOTERRAINPOP
        f_pos.z -= pull_down;
    #else
        f_pos.z -= step(0.1, pull_down) * 10000.0;
    #endif

    #ifdef EXPERIMENTAL_CURVEDWORLD
        f_pos.z -= pow(distance(f_pos.xy + focus_off.xy, focus_pos.xy + focus_off.xy) * 0.05, 2);
    #endif

    // Hacky, very bad, 50 is ~ tree height
    f_norm = v_norm;//mix(v_norm, vec3(0, 0, 1), clamp(model_pos.z / 50, 0, 1));
    f_col = vec4(vec3(0.02, 0.1, 0.01) * (sin(inst_pos.xyy) * 0.33 + 0.66), 1.0);//vec4(v_col, 1.0);

    if ((inst_flags & FLAG_SNOW_COVERED) > 0u) {
        snow_cover = 1.0;
        //f_col.rgb = mix(f_col.rgb, vec3(1), pow(max(f_norm.z, 0.0), 0.5));
    } else {
        snow_cover = 0.0;
    }

    gl_Position =
        all_mat *
        vec4(f_pos, 1);
}
