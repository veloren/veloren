#include <lod.glsl>

vec3 get_cloud_color(vec3 surf_color, vec3 dir, vec3 origin, float time_of_day, float max_dist, float quality) {
    // Underwater light attenuation
    surf_color = water_diffuse(surf_color, dir, max_dist);

    return surf_color;
}
