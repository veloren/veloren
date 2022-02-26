#ifndef POINT_GLOW_GLSL
#define POINT_GLOW_GLSL

vec3 apply_point_glow(vec3 wpos, vec3 dir, float max_dist, vec3 color) {
    #ifndef POINT_GLOW_FACTOR
        return color;
    #else
        for (uint i = 0u; i < light_shadow_count.x; i ++) {
            // Only access the array once
            Light L = lights[i];

            vec3 light_pos = L.light_pos.xyz;
            // Project light_pos to dir line
            float t = max(dot(light_pos - wpos, dir), 0);
            vec3 nearest = wpos + dir * min(t, max_dist);

            vec3 difference = light_pos - nearest;
            float distance_2 = dot(difference, difference);
            if (distance_2 > 100000.0) {
                continue;
            }

            #if (CLOUD_MODE >= CLOUD_MODE_HIGH)
                vec3 _unused;
                float unused2;
                float spread = 1.0 / (1.0 + cloud_at(nearest, 0.0, _unused, unused2).z * 0.005);
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
                // Constant, *should* const fold
                * POINT_GLOW_FACTOR;
        }
    #endif
    return color;
}

#endif
