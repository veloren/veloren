#include <lod.glsl>
#include <sky.glsl>

vec3 get_cloud_color(vec3 surf_color, vec3 dir, vec3 origin, float time_of_day, float max_dist, float quality) {
    // Underwater light attenuation
    surf_color = water_diffuse(surf_color, dir, max_dist);

    if (max_dist < DIST_CAP) {
        vec3 sky_light = get_sky_light(dir, time_of_day, false);
        surf_color = mix(sky_light, surf_color, 1.0 / exp(max_dist / 5000.0));
    }

    return surf_color;
}
