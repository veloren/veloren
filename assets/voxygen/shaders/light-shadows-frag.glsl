// NOTE: We currently do nothing, and just rely on the default shader behavior.
//
// However, in the future we might apply some depth transforms here.

#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#if (FLUID_MODE == FLUID_MODE_CHEAP)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE == FLUID_MODE_SHINY)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

// Currently, we only need globals for the far plane.
#include <globals.glsl>
// Currently, we only need lights for the light position
#include <light.glsl>

in vec4 FragPos; // FragPos from GS (output per emitvertex)
flat in int FragLayer;

void main()
{
    // get distance between fragment and light source
    float lightDistance = length(FragPos.xyz - lights[FragLayer & 31].light_pos.xyz);

    // map to [0;1] range by dividing by far_plane
    lightDistance = lightDistance / /*FragPos.w;*/screen_res.w;

    // write this as modified depth
    gl_FragDepth = lightDistance;
}
