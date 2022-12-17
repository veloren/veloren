#ifndef POINT_GLOW_GLSL
#define POINT_GLOW_GLSL

#include "sky.glsl"

void apply_point_glow_light(Light L, vec3 wpos, vec3 dir, float max_dist, inout vec3 color) {
    vec3 light_pos = L.light_pos.xyz;
    // Project light_pos to dir line
    float t = max(dot(light_pos - wpos, dir), 0);
    vec3 nearest = wpos + dir * min(t, max_dist);

    vec3 difference = light_pos - nearest;
    float distance_2 = dot(difference, difference);
    //if (distance_2 > 100000.0) {
    //    return;
    //}

    #if (CLOUD_MODE >= CLOUD_MODE_HIGH)
        vec3 _unused;
        float unused2;
        float spread = 1.0 / (1.0 + sqrt(cloud_at(nearest, 0.0, _unused, unused2).z) * 0.01);
    #else
        const float spread = 1.0;
    #endif

    float strength = pow(attenuation_strength_real(difference), spread);

    #ifdef EXPERIMENTAL_LOWGLOWNEARCAMERA
        vec3 cam_wpos = cam_pos.xyz + focus_pos.xyz + focus_off.xyz;
        vec3 cam_diff = light_pos - cam_wpos;
        float cam_dist_2 = dot(cam_diff, cam_diff);
        // 3 meters away glow returns to the maximum strength.
        strength *= clamp(cam_dist_2 / 9.0, 0.25, 1.0);
    #endif

    vec3 light_color = srgb_to_linear(L.light_col.rgb) * strength;

    const float LIGHT_AMBIANCE = 0.025;
    color += light_color
        * 0.002
    #ifdef POINT_GLOW_FACTOR
        // Constant, *should* const fold
        * POINT_GLOW_FACTOR
    #endif
    ;
}

vec3 apply_point_glow(vec3 wpos, vec3 dir, float max_dist, vec3 color) {
    #ifndef POINT_GLOW_FACTOR
        return color;
    #else
        for (uint i = 0u; i < light_shadow_count.x; i ++) {
            // Only access the array once
            Light L = lights[i];

            apply_point_glow_light(L, wpos, dir, max_dist, color);
        }
    #endif

    #ifdef FLASHING_LIGHTS_ENABLED
        float time_since_lightning = tick.x - last_lightning.w;
        if (time_since_lightning < MAX_LIGHTNING_PERIOD) {
            // Apply lightning
            apply_point_glow_light(Light(last_lightning.xyzw + vec4(0, 0, LIGHTNING_HEIGHT, 0), vec4(vec3(0.2, 0.4, 1) * lightning_intensity() * 0.003, 1)), wpos, dir, max_dist, color);
        }
    #endif
    return color;
}

#endif
