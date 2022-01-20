#ifndef POINT_GLOW_GLSL
#define POINT_GLOW_GLSL

vec3 apply_point_glow(vec3 wpos, vec3 dir, float max_dist, vec3 color, const float factor) {
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

            vec3 light_color = srgb_to_linear(L.light_col.rgb) * strength * L.light_col.a;

            const float LIGHT_AMBIANCE = 0.025;
            color += light_color
                * 0.05
                // Constant, *should* const fold
                * pow(factor, 0.65)
                * POINT_GLOW_FACTOR;
        }
    #endif
    return color;
}

#endif
