// Adapted from https://learnopengl.com/Advanced-Lighting/Shadows/Point-Shadows

// NOTE: We only technically need this for cube map arrays and geometry shader
// instancing.
#version 330 core
// #extension ARB_texture_storage : enable

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

#define HAS_SHADOW_MAPS

// Currently, we only need globals for the max light count (light_shadow_count.x)
// and the far plane (scene_res.z).
#include <globals.glsl>
#include <shadows.glsl>
// // Currently, we only need lights for the light position
// #include <light.glsl>

/* struct Light {
	vec4 light_pos;
	vec4 light_col;
    // mat4 light_proj;
};

layout (std140)
uniform u_lights {
	Light lights[31];
}; */

// Since our output primitive is a triangle strip, we have to render three vertices
// each.
#define VERTICES_PER_FACE 3

// Since we render our depth texture to a cube map, we need to render each face
// six times.  If we used other shadow mapping methods with fewer outputs, this would
// shrink considerably.
#define FACES_PER_POINT_LIGHT 6

// If MAX_VERTEX_UNIFORM_COMPONENTS_ARB = 512 on many platforms, and we want a mat4
// for each of 6 directions for each light, 20 is close to the maximum allowable
// size.  We could add a final matrix for the directional light of the sun or moon
// to bring us to 126 matrices, which is just 2 off.
//
// To improve this limit, we could do many things, such as:
// - choose an implementation that isn't cube maps (e.g. tetrahedrons or curves;
//   if there were an easy way to sample from tetrahedrons, we'd be at 32 * 4 = 128
//   exactly, leaving no room for a solar body, though).
// - Do more work in the geometry shader (e.g. just have a single projection
//   matrix per light, and derive the different-facing components; or there may be
//   other ways of greatly simplifying this).  The tradeoff would be losing performance
//   here.
// - Use ARB_instanced_arrays and switch lights with indexing, instead of a uniform
//   buffer.  This would probably work fine (and ARB_instanced_arrays is supported on
//   pretty much every platform), but AFAIK it's possible that instanced arrays are
//   slower than uniform arraay access on many platforms.
// - Don't try to do everything in one call (break this out into multiple passes).
//
// Actually, according to what I'm reading, MAX_GEOM_UNIFORM_COMPONENTS = 1024, and
// gl_MaxGeometryUniformComponents = 1024.
//
// Also, this only applies to uniforms defined *outside* of uniform blocks, of which
// there can be up to 12 (14 in OpenGL 4.3, which we definitely can't support).
// GL_MAX_UNIFORM_BLOCK_SIZE has a minimum of 16384, which *easily* exceeds our usage
// constraints. So this part might not matter.
//
// Other restrictions are easy to satisfy:
//
// gl_MaxGeometryVaryingComponents has a minimum of 64 and is the maximum number of
// varying components; I think this is the number of out components per vertex, which
// is technically 0, but would be 4 if we wrote FragPos.  But it might also
// be the *total* number of varying components, in which case if we wrote FragPos
// it would be 4 * 20 * 6 * 3 = 1440, which would blow it out of the water.  However,
// I kind of doubt this interpretation because writing FragPos for each of 18 vertices,
// as the original shader did, already yields 4 * 18 = 72, and it seems unlikely that
// the original example exceeds OpenGL limits.
//
// gl_MaxGeometryOutputComponents has a minimum of 128 and is the maximum number of
// components allowed in out variables; we easily fall under this since we actually
// have 0 of these.  However, if we were to write FragPos for each vertex, it *might*
// cause us to exceed this limit, depending on whether it refers to the total output
// component count *including* varying components, or not.  See the previous
// discussion; since 72 < 128 it's more plausible that this interpretation might be
// correct, but hopefully it's not.
//
// gl_MaxGeometryInputComponents has a minimum of 64 and we easily fall under that
// limit (I'm actually not sure we even have any user-defined input components?).
//
// gl_MaxGeometryTextureImageUnits = 16 and we have no texture image units (or maybe
// 1, the one we bound?).  This might come into play if we were to have attached
// cubemaps instead of a single cubemap array, in which case it would limit us to
// 16 lights *regardless* of any of the fixes mentioned above (i.e., we'd just have
// to split up draw calls, I think).
//
// ---
//
// However, there is another limit to consider: GL_MAX_GEOMETRY_OUTPUT_VERTICES.  Its
// minimum is 256, and 20 * 6 * 3 = 360, which exceeds that.  This introduces a new
// limit of at most 14 point lights.
//
// Another, related limit is GL_MAX_GEOMETRY_TOTAL_OUTPUT_COMPONENTS.  This counts
// every component output ("component" is usually a 4-byte field of a vector, but maybe
// this would improve with something like half-floats?), and has a minimum (as of
// OpenGL 3.3) of 1024.  Since even builtin outputs gl_Layer count against this total,
// this means we issue 5 components per vertex, and 14 * 6 * 3 * 5 = 1260 > 1024.
//
// Ultimately, we find our maximum output limit of 11, ≤ 1024/5/3/6.
//
// If we choose to reserve a slot for a non-point light (and/or other uniforms), it
// is just 10, or half what we got from VERTICES_PER_FACE (we could also round down to
// 8 as a power of 2, if we had to).
//
// Unlike the input limits, whwich we can get around with "clever" solutions, it seems
// likely that the only real way to defeat the vertex limits is to use instancing of
// some sort (be it geometry shader or otherwise).  This would restrict us to OpenGL
// 4.0 or above.
//
// A further consideration (were we to switch to OpenGL 4.1-supported features, but
// actually it is often supported on 3.3 hardware with ARB_viewport_array--whereas
// geometry shader instancing is *not* supported on any 3.3 hardware, so would actually
// require us to upgrade) would be setting gl_ViewportIndex.  The main reason to consider
// this is that it allows specifying a separate scissor rectangle per viewport.  This
// introduces two new constraints.  Firstly, it adds an extra component to each vertex
// (lowering our maximum to 9 faces per light ≤ 1024/6/3/6, or 8 if we want to support a
// directional light).
//
// Secondly, a new constant (MAX_VIEWPORTS) is introduced, which would restrict the
// total number of active viewports; the minimum value for this is 16.  While this may
// not seem all that relevant since our current hard limit is 11, the difference is that
// this limit would apply *across* instanced calls (since it may be a "global"
// restriction, tied to the OpenGL context; this means it couldn't even be a multiple
// frame buffer thing, as there is usually one per window).  This would also tie in
// with gl_MaxGeometryTextureImageUnits, I guess.
//
// --
//
// I just realized tht using cube map arrays at all bumps our required OpenGL
// version to 4.0, so let's just do instancing...
//
// The instancing limit on MAX_GEOMETRY_SHADER_INVOCATIONS has a minimum of 32, which
// would be sufficient to run through all 32 lights with a different cube map and
// completely removes any imits on ight count.
//
// This should instantly bring us below all relevant limits in all cases considered
// except for the two that would require 16.  Unfortunately, 32 is also the *maximum*
// number of point lights, which is much higher than the usual value, and the instance
// count has to be a constant.  If we were to instead geometry-shader-instance each
// *face*, we'd get a maximum light count of 56 ≤ 1024/6/3, which is not as elegant
// but is easily higher than 32.  So, let's try using that instead.
//
// It is *possible* that using instancing on the *vertex* shader with the (dynamically
// uniform) total number of instances set to the actual number of point lights, would
// improve performance, since it would give us a 1:1 vertex input:output ratio, which
// might be optimized in hardware.
//
// It also seems plausible that constructing a separate geometry shader with values
// from 1 to 32 would be worthwhile, but that seems a little extreme.
//
// ---
//
// Since wgpu doesn't support geometry shaders anyway, it seems likely that we'll have
// to do the multiple draw calls, anyway... I don't think gl_Layer can be set from
// outside a geometry shader.  But in wgpu, such a thing is much cheaper, anyway.
#define MAX_POINT_LIGHTS 31

// We use geometry shader instancing to construct each face separately.
#define MAX_LAYER_VERTICES_PER_FACE (MAX_POINT_LIGHTS * VERTICES_PER_FACE)

#define MAX_LAYER_FACES (MAX_POINT_LIGHTS * FACES_PER_POINT_LIGHT)

layout (triangles/*, invocations = 6*/) in;

layout (triangle_strip, max_vertices = /*MAX_LAYER_VERTICES_PER_FACE*//*96*/18) out;

//struct ShadowLocals {
//	mat4 shadowMatrices;
//    mat4 texture_mat;
//};
//
//layout (std140)
//uniform u_light_shadows {
//    ShadowLocals shadowMats[/*MAX_LAYER_FACES*/192];
//};

// NOTE: We choose not to output FragPos currently to save on space limitations
// (see extensive documentation above).  However, as these limitations have been
// relaxed (unless the total of all our varying output components can't exceed
// 128, which would mean FragPos would sum to 4 * 3 * 32 = 384; this could be
// remedied only by setting MAX_POINT_LIGHTS to ), we might enable it again soon.
//
// out vec3 FragPos; // FragPos from GS (output per emitvertex)
// flat out int FragLayer; // Current layer

// const vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));

void main() {
    // return;
    // NOTE: Assuming that light_shadow_count.x < MAX_POINT_LIGHTS.  We could min
    // it, but that might make this less optimized, and I'd like to keep this loop as
    // optimized as is reasonably possible.
    // int face = gl_InvocationID;

    // Part 1: emit directed lights.
    /* if (face <= light_shadow_count.z) {
        // Directed light.
        for(int i = 0; i < VERTICES_PER_FACE; ++i) // for each triangle vertex
        {
            // NOTE: See above, we don't make FragPos a uniform.
            FragPos = gl_in[i].gl_Position;
            FragLayer = 0; // 0 is the directed light layer.
            // vec4 FragPos = gl_in[i].gl_Position;
            gl_Layer = i; // built-in variable that specifies to which face we render.
            gl_Position = shadowMats[i].shadowMatrices * FragPos;
            EmitVertex();
        }
        EndPrimitive();
    } */

    // Part 2: emit point lights.
    /* if (light_shadow_count.x == 1) {
        return;
    } */
#if (SHADOW_MODE == SHADOW_MODE_MAP)
    for (uint layer = 1u; layer <= min(light_shadow_count.x, 1u); ++layer)
    {
        int layer_base = int(layer) * FACES_PER_POINT_LIGHT;
        // We use instancing here in order to increase the number of emitted vertices.
        // int face = gl_InvocationID;
        for(int face = 0; face < FACES_PER_POINT_LIGHT; ++face)
        {
            // int layer_face = layer * FACES_PER_POINT_LIGHT + face;
            // int layer_face = layer * FACES_PER_POINT_LIGHT + face;
            // for(int i = VERTICES_PER_FACE - 1; i >= 0; --i) // for each triangle vertex
            for(int i = 0; i < VERTICES_PER_FACE; ++i) // for each triangle vertex
            {
                // NOTE: See above, we don't make FragPos a uniform.
                vec3 fragPos = gl_in[i].gl_Position.xyz;
                // FragPos = fragPos - (lights[((/*FragLayer*/layer - 1u) & 31u)].light_pos.xyz - focus_off.xyz);
                // FragLayer = layer;
                // float lightDistance = length(FragPos - lights[((layer - 1) & 31)].light_pos.xyz);
                // lightDistance /= screen_res.w;

                // vec4 FragPos = gl_in[i].gl_Position;
                // NOTE: Our normals map to the same thing as cube map normals, *except* that their normal direction is
                // swapped; we can fix this by doing normal ^ 0x1u.  However, we also want to cull back faces, not front
                // faces, so we only care about the shadow cast by the *back* of the triangle, which means we ^ 0x1u
                // again and cancel it out.
                // int face = int(((floatBitsToUint(gl_Position.w) >> 29) & 0x7u) ^ 0x1u);
                int layer_face = layer_base + face;
                gl_Layer = face;//layer_face; // built-in variable that specifies to which face we render.
                gl_Position = shadowMats[layer_face].shadowMatrices * vec4(fragPos, 1.0);
                // gl_Position.z = -((gl_Position.z + screen_res.z) / (screen_res.w - screen_res.z)) * lightDistance;
                // gl_Position.z = gl_Position.z / screen_res.w;
                // gl_Position.z = gl_Position.z / gl_Position.w;
                // gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
                // lightDistance = -(lightDistance + screen_res.z) / (screen_res.w - screen_res.z);
                // gl_Position.z = lightDistance;
                EmitVertex();
            }
            EndPrimitive();
         }
    }
#endif
}
