#version 420 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_TRANSMISSION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_SPECULAR

#if (FLUID_MODE == FLUID_MODE_CHEAP)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE == FLUID_MODE_SHINY)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>

layout(location = 0) in vec3 v_pos;

layout(location = 0) out vec3 f_pos;

void main() {
    f_pos = v_pos;

    // TODO: is this todo below still valid? is cam_pos jittery
    // TODO: Make this position-independent to avoid rounding error jittering
    gl_Position =
        all_mat *
        vec4(v_pos + cam_pos.xyz, 1);
    // gl_Position = vec4(gl_Position.xy, sign(gl_Position.z) * gl_Position.w, gl_Position.w);
    gl_Position.z = 0;
    // gl_Position.z = gl_Position.w - 0.000001;//0.0;
    // gl_Position.z = 1.0;
    // gl_Position.z = -1.0;
}
