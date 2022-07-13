
#ifndef RAIN_OCCLUSION_GLSL
#define RAIN_OCCLUSION_GLSL

// Use with sampler2DShadow
layout(set = 1, binding = 4)
uniform texture2D t_directed_occlusion_maps;
layout(set = 1, binding = 5)
uniform samplerShadow s_directed_occlusion_maps;

layout (std140, set = 0, binding = 14)
uniform u_rain_occlusion {
    mat4 rain_occlusion_matrices;
    mat4 rain_occlusion_texture_mat;
    mat4 rain_dir_mat;
    float integrated_rain_vel;
    float rain_density;
    vec2 occlusion_dummy; // Fix alignment.
};

float rain_occlusion_at(in vec3 fragPos)
{
    vec4 rain_pos = rain_occlusion_texture_mat * vec4(fragPos, 1.0);

    float visibility = textureProj(sampler2DShadow(t_directed_occlusion_maps, s_directed_occlusion_maps), rain_pos);

    return visibility;
}
#endif
