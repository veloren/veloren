#ifndef POINT_GLOW_GLSL
#define POINT_GLOW_GLSL

#include <sky.glsl>

void apply_point_glow_light(Light L, vec3 wpos, vec3 dir, float max_dist, inout vec3 color) {
    vec3 light_pos = L.light_pos.xyz;
    // Project light_pos to dir line
    float t = max(dot(light_pos - wpos, dir), 0);
    vec3 nearest = wpos + dir * min(t, max_dist);

    vec3 difference = light_pos - nearest;
    float distance_2 = dot(difference, difference);

    #if (CLOUD_MODE >= CLOUD_MODE_HIGH)
        vec3 _unused;
        float unused2;
        float spread = 1.0 / (1.0 + sqrt(max(cloud_at(nearest, 0.0, dir, _unused, unused2).z, 0.0)) * 0.01);
    #else
        const float spread = 1.0;
    #endif

    float strength = 0.0;
    // Directional lights
    if (L.light_dir.w < 1.0) {
        // Base ambient light
        strength += pow(attenuation_strength_real(difference), spread)
            // A more focussed beam means less ambiance
            * (1.0 - L.light_dir.w);
        // Compute intersection of directional ray with light cone
        const vec3 ldir = L.light_dir.xyz;
        const vec3 beam_origin = light_pos - ldir * 0.1;
        const float y2 = L.light_dir.w * L.light_dir.w;
        const float c2 = pow(dot(ldir, dir), 2.0) - y2 * dot(dir, dir);
        const float c1 = dot(ldir, dir) * dot(ldir, wpos - beam_origin) - y2 * dot(dir, wpos - beam_origin);
        const float c0 = pow(dot(ldir, wpos - beam_origin), 2.0) - y2 * dot(wpos - beam_origin, wpos - beam_origin);
        const float roots = c1 * c1 - c0 * c2;
        if (roots >= 0) {
            const float t0 = ((-c1 + sqrt(roots)) / c2);
            const float t1 = ((-c1 - sqrt(roots)) / c2);
            const float t = min(dot(dir, ldir) < 0.0 ? t1 : t0, max_dist);
            if (t > 0.0 && dot(normalize(wpos + dir * t - beam_origin), ldir) + 0.01 > L.light_dir.w) {
                nearest = wpos + dir * t;
                if (dot(nearest - beam_origin, ldir) > 0.0) {
                    difference = light_pos - nearest;
                    float power = clamp(pow(abs(t0 - t1), 2.5) / length(difference), 0.0, 1.0);
                    strength += pow(attenuation_strength_real(difference), spread)
                        * (2.0 + dot(ldir, -dir))
                        * power;
                }
            }
        }
    } else {
        // Regular lights
        strength = pow(attenuation_strength_real(difference), spread);
    }

    #ifdef EXPERIMENTAL_LOWGLOWNEARCAMERA
        vec3 cam_wpos = cam_pos.xyz + focus_pos.xyz + focus_off.xyz;
        vec3 cam_diff = light_pos - cam_wpos;
        float cam_dist_2 = dot(cam_diff, cam_diff);
        // 3 meters away glow returns to the maximum strength.
        strength *= clamp(cam_dist_2 / 9.0, 0.25, 1.0);
    #endif

    vec3 light_color = L.light_col.rgb * strength;

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

        #ifdef FLASHING_LIGHTS_ENABLED
            float time_since_lightning = time_since(last_lightning.w);
            if (time_since_lightning < MAX_LIGHTNING_PERIOD) {
                // Apply lightning
                apply_point_glow_light(
                    Light(last_lightning.xyzw + vec4(0, 0, LIGHTNING_HEIGHT, 0), vec4(vec3(0.2, 0.4, 1) * lightning_intensity() * 0.003, 1), vec4(vec3(0.0), 10.0)),
                    wpos,
                    dir,
                    max_dist,
                    color
                );
            }
        #endif
        return color;
    #endif
}

#endif
