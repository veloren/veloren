#version 420 core

#include <constants.glsl>

#define LIGHTING_TYPE (LIGHTING_TYPE_TRANSMISSION | LIGHTING_TYPE_REFLECTION)

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_SPECULAR

#if (FLUID_MODE == FLUID_MODE_LOW)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE >= FLUID_MODE_MEDIUM)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>

layout(location = 0) out vec2 uv;

void main() {
    // Generate fullscreen triangle
    vec2 v_pos = vec2(
        float(gl_VertexIndex / 2) * 4.0 - 1.0,
        float(gl_VertexIndex % 2) * 4.0 - 1.0
    );

    // Flip y and transform into 0.0 to 1.0 range
    uv = (v_pos * vec2(1.0, -1.0) + 1.0) * 0.5;

    gl_Position = vec4(v_pos, 0.0, 1.0);
}
