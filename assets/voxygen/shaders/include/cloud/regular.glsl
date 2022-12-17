#include <constants.glsl>
#include <random.glsl>
#include <light.glsl>
#include <lod.glsl>

float falloff(float x) {
    return pow(max(x > 0.577 ? (0.3849 / x - 0.1) : (0.9 - x * x), 0.0), 4);
}

float billow_noise_3d(vec3 pos) {
    return abs(noise_3d(pos) - 0.5) * 2.0;
}

float billow_noise_2d(vec2 pos) {
    return abs(noise_2d(pos) - 0.5) * 2.0;
}

// Returns vec4(r, g, b, density)
vec4 cloud_at(vec3 pos, float dist, out vec3 emission, out float not_underground) {
    #ifdef EXPERIMENTAL_CURVEDWORLD
        pos.z += pow(distance(pos.xy, focus_pos.xy + focus_off.xy) * 0.05, 2);
    #endif

    // Natural attenuation of air (air naturally attenuates light that passes through it)
    // Simulate the atmosphere thinning as you get higher. Not physically accurate, but then
    // it can't be since Veloren's world is flat, not spherical.
    float atmosphere_alt = CLOUD_AVG_ALT + 40000.0;
    // Veloren's world is flat. This is, to put it mildly, somewhat non-physical. With the earth as an infinitely-big
    // plane, the atmosphere is therefore capable of scattering 100% of any light source at the horizon, no matter how
    // bright, because it has to travel through an infinite amount of atmosphere. This doesn't happen in reality
    // because the earth has curvature and so there is an upper bound on the amount of atmosphere that a sunset must
    // travel through. We 'simulate' this by fading out the atmosphere density with distance.
    float flat_earth_hack = 1.0 / (1.0 + dist * 0.0001);
    float air = 0.025 * clamp((atmosphere_alt - pos.z) / 20000, 0, 1) * flat_earth_hack;

    float alt = alt_at(pos.xy - focus_off.xy);

    // Mist sits close to the ground in valleys (TODO: use base_alt to put it closer to water)
    float mist_min_alt = 0.5;
    #if (CLOUD_MODE >= CLOUD_MODE_MEDIUM)
        mist_min_alt = (textureLod(sampler2D(t_noise, s_noise), pos.xy / 35000.0, 0).x - 0.5) * 1.5 + 0.5;
    #endif
    mist_min_alt = view_distance.z * 1.5 * (1.0 + mist_min_alt * 0.5) + alt * 0.5 + 250;
    const float MIST_FADE_HEIGHT = 1000;
    float mist = 0.01 * pow(clamp(1.0 - (pos.z - mist_min_alt) / MIST_FADE_HEIGHT, 0.0, 1), 10.0) * flat_earth_hack;

    vec3 wind_pos = vec3(pos.xy + wind_offset, pos.z + noise_2d(pos.xy / 20000) * 500);

    // Clouds
    float cloud_tendency = cloud_tendency_at(pos.xy);
    float cloud = 0;

    if (mist > 0.0) {
        mist *= 0.5
        #if (CLOUD_MODE >= CLOUD_MODE_LOW)
            + 1.0 * (noise_2d(wind_pos.xy / 5000) - 0.5)
        #endif
        #if (CLOUD_MODE >= CLOUD_MODE_MEDIUM)
            + 0.25 * (noise_3d(wind_pos / 1000) - 0.5)
        #endif
        ;
    }

    float CLOUD_DEPTH = (view_distance.w - view_distance.z) * (0.2 + sqrt(cloud_tendency) * 0.5);
    float cloud_alt = alt + CLOUD_DEPTH * 2 + 1000.0;

    //vec2 cloud_attr = get_cloud_heights(wind_pos.xy);
    float sun_access = 0.0;
    float moon_access = 0.0;
    float cloud_sun_access = clamp((pos.z - cloud_alt) / 1500 + 0.5, 0, 1);
    float cloud_moon_access = 0.0;

    // This is a silly optimisation but it actually nets us a fair few fps by skipping quite a few expensive calcs
    if ((pos.z < CLOUD_AVG_ALT + 8000.0 && cloud_tendency > 0.0)) {
        // Turbulence (small variations in clouds/mist)
        const float turb_speed = -1.0; // Turbulence goes the opposite way
        vec3 turb_offset = vec3(1, 1, 0) * time_of_day.x * turb_speed;

        const float CLOUD_DENSITY = 10000.0;
        const float CLOUD_ALT_VARI_WIDTH = 100000.0;
        const float CLOUD_ALT_VARI_SCALE = 5000.0;

        float small_nz = 0.0
        #if (CLOUD_MODE >= CLOUD_MODE_MEDIUM)
            - (billow_noise_3d((pos + turb_offset * 0.5) / 8000.0) - 0.5)
        #else
            - (billow_noise_2d((pos.xy + turb_offset.xy * 0.5) / 8000.0) - 0.5)
        #endif
        #if (CLOUD_MODE >= CLOUD_MODE_CLOUD_MODE_MINIMAL)
            - (noise_3d((pos - turb_offset * 0.1) / 750.0) - 0.5) * 0.25
        #endif
        #if (CLOUD_MODE >= CLOUD_MODE_CLOUD_MODE_HIGH)
            - (billow_noise_3d((pos - turb_offset * 0.1) / 500.0) - 0.5) * 0.1
        #endif
        ;

        // Sample twice to allow for self-shadowing
        float cloud_p0 = noise_3d((wind_pos + vec3(0, 0, small_nz) * 250 - sun_dir.xyz * 250) * vec3(0.55, 0.55, 1) / (cloud_scale * 20000.0));
        float cloud_p1 = noise_3d((wind_pos + vec3(0, 0, small_nz) * 250 + sun_dir.xyz * 250) * vec3(0.55, 0.55, 1) / (cloud_scale * 20000.0));

        float cloud_factor = pow(max(((cloud_p0 + cloud_p1) * 0.5
            - 0.5
            - small_nz * 0.1
            + cloud_tendency * 0.3
            )
        , 0.0) * 120.0 * cloud_tendency, 5.0)
            * falloff(abs(pos.z - cloud_alt) / CLOUD_DEPTH);

        cloud = cloud_factor * 10;

        // What proportion of sunlight is *not* being blocked by nearby cloud? (approximation)
        // Basically, just throw together a few values that roughly approximate this term and come up with an average
        cloud_sun_access = clamp(
            0.7
                + pow(abs(cloud_p1 - cloud_p0), 0.5) * sign(cloud_p1 - cloud_p0) * 0.5
                + (pos.z - cloud_alt) / CLOUD_DEPTH * 0.4
                - pow(cloud * 10000000.0, 0.2) * 0.0075
            ,
            0.15,
            10.0
        ) + small_nz * 0.2;
        // Since we're assuming the sun/moon is always above (not always correct) it's the same for the moon
        cloud_moon_access = cloud_sun_access;
    }

    float mist_sun_access = max(1.0 - cloud_tendency * 8, 0.25);
    float mist_moon_access = mist_sun_access;
    sun_access = mix(cloud_sun_access, mist_sun_access, clamp(mist * 20000, 0, 1));
    moon_access = mix(cloud_moon_access, mist_moon_access, clamp(mist * 20000, 0, 1));

    // Prevent mist (i.e: vapour beneath clouds) being accessible to the sun to avoid visual problems
    //float suppress_mist = clamp((pos.z - cloud_attr.x + cloud_attr.y) / 300, 0, 1);
    //sun_access *= suppress_mist;
    //moon_access *= suppress_mist;

    // Prevent clouds and mist appearing underground (but fade them out gently)
    not_underground = clamp(1.0 - (alt - (pos.z - focus_off.z)) / 80.0 + dist * 0.001, 0, 1);
    sun_access *= not_underground;
    moon_access *= not_underground;
    float vapor_density = (mist + cloud) * not_underground;

    if (emission_strength <= 0.0) {
        emission = vec3(0);
    } else {
        float nz = textureLod(sampler2D(t_noise, s_noise), wind_pos.xy * 0.00005 - time_of_day.x * 0.0001, 0).x;//noise_3d(vec3(wind_pos.xy * 0.00005 + cloud_tendency * 0.2, time_of_day.x * 0.0002));

        float emission_alt = alt * 0.5 + 1000 + 1000 * nz;
        float emission_height = 1000.0;
        float emission_factor = pow(max(0.0, 1.0 - abs((pos.z - emission_alt) / emission_height - 1.0))
            * max(0, 1.0 - abs(0.0
                + textureLod(sampler2D(t_noise, s_noise), wind_pos.xy * 0.0001 + nz * 0.1, 0).x
                + textureLod(sampler2D(t_noise, s_noise), wind_pos.xy * 0.0005 + nz * 0.5, 0).x * 0.3
                - 0.5) * 2)
            * max(0, 1.0 - abs(textureLod(sampler2D(t_noise, s_noise), wind_pos.xy * 0.00001, 0).x - 0.5) * 4)
            , 2) * emission_strength;
        float t = clamp((pos.z - emission_alt) / emission_height, 0, 1);
        t = pow(t - 0.5, 2) * sign(t - 0.5) + 0.5;
        float top = pow(t, 2);
        float bot = pow(max(0.8 - t, 0), 2) * 2;
        const vec3 cyan = vec3(0, 0.5, 1);
        const vec3 red = vec3(1, 0, 0);
        const vec3 green = vec3(0, 8, 0);
        emission = 10 * emission_factor * nz * (cyan * top * max(0, 1 - emission_br) + red * max(emission_br, 0) + green * bot);
    }

    // We track vapor density and air density separately. Why? Because photons will ionize particles in air
    // leading to rayleigh scattering, but water vapor will not. Tracking these indepedently allows us to
    // get more correct colours.
    return vec4(sun_access, moon_access, vapor_density, air);
}

#if (CLOUD_MODE == CLOUD_MODE_ULTRA)
    const uint QUALITY = 200u;
#elif (CLOUD_MODE == CLOUD_MODE_HIGH)
    const uint QUALITY = 40u;
#elif (CLOUD_MODE == CLOUD_MODE_MEDIUM)
    const uint QUALITY = 18u;
#elif (CLOUD_MODE == CLOUD_MODE_LOW)
    const uint QUALITY = 6u;
#elif (CLOUD_MODE == CLOUD_MODE_MINIMAL)
    const uint QUALITY = 2u;
#endif

const float STEP_SCALE = DIST_CAP / (10.0 * float(QUALITY));

float step_to_dist(float step, float quality) {
    return pow(step, 2) * STEP_SCALE / quality;
}

float dist_to_step(float dist, float quality) {
    return pow(dist / STEP_SCALE * quality, 0.5);
}

// This *MUST* go here: when clouds are enabled, it relies on the declaration of `clouds_at` above. Sadly, GLSL doesn't
// consistently support forward declarations (not surprising, it's designed for single-pass compilers).
#include <point_glow.glsl>

vec3 get_cloud_color(vec3 surf_color, vec3 dir, vec3 origin, const float time_of_day, float max_dist, const float quality) {
    // Limit the marching distance to reduce maximum jumps
    max_dist = min(max_dist, DIST_CAP);

    origin.xyz += focus_off.xyz;

    // This hack adds a little direction-dependent noise to clouds. It's not correct, but it very cheaply
    // improves visual quality for low cloud settings
    float splay = 1.0;
    #if (CLOUD_MODE == CLOUD_MODE_MINIMAL)
        splay += (textureLod(sampler2D(t_noise, s_noise), vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 5.0 - time_of_day * 0.00005, 0).x - 0.5) * 0.025 / (1.0 + pow(dir.z, 2) * 10);
    #endif

    const vec3 RAYLEIGH = vec3(0.025, 0.1, 0.5);

    // Proportion of sunlight that get scattered back into the camera by clouds
    float sun_scatter = dot(-dir, sun_dir.xyz) * 0.5 + 0.7;
    float moon_scatter = dot(-dir, moon_dir.xyz) * 0.5 + 0.7;
    float net_light = get_sun_brightness() + get_moon_brightness();
    vec3 sky_color = RAYLEIGH * net_light;
    vec3 sky_light = get_sky_light(dir, time_of_day, false);
    vec3 sun_color = get_sun_color();
    vec3 moon_color = get_moon_color();

    // Clouds aren't visible underwater
    float cdist = max_dist;
    float ldist = cdist;
    // i is an emergency brake
    float min_dist = clamp(max_dist / 4, 0.25, 24);
    int i;

    #if (CLOUD_MODE >= CLOUD_MODE_MEDIUM)
    #ifndef EXPERIMENTAL_NORAINBOWS
        // TODO: Make it a double rainbow
        float rainbow_t = (0.7 - dot(sun_dir.xyz, dir)) * 8 / 0.05;
        int rainbow_c = int(floor(rainbow_t));
        rainbow_t = fract(rainbow_t);
        rainbow_t = rainbow_t * rainbow_t;
    #endif
    #endif

    for (i = 0; cdist > min_dist && i < 250; i ++) {
        ldist = cdist;
        cdist = step_to_dist(trunc(dist_to_step(cdist - 0.25, quality)), quality);

        vec3 emission;
        float not_underground; // Used to prevent sunlight leaking underground
        vec3 pos = origin + dir * ldist * splay;
        // `sample` is a reserved keyword
        vec4 sample_ = cloud_at(origin + dir * ldist * splay, ldist, emission, not_underground);

        // DEBUG
        // if (max_dist > ldist && max_dist < ldist * 1.02) {
        //     surf_color = vec3(1, 0, 0);
        // }

        vec2 density_integrals = max(sample_.zw, vec2(0));

        float sun_access = max(sample_.x, 0);
        float moon_access = max(sample_.y, 0);
        float cloud_scatter_factor = density_integrals.x;
        float global_scatter_factor = density_integrals.y;

        float step = (ldist - cdist) * 0.01;
        float cloud_darken = pow(1.0 / (1.0 + cloud_scatter_factor), step);
        float global_darken = pow(1.0 / (1.0 + global_scatter_factor), step);
        // Proportion of light diffusely scattered instead of absorbed
        float cloud_diffuse = 0.5;

        surf_color =
            // Attenuate light passing through the clouds
            surf_color * cloud_darken * global_darken +
            // Add the directed light light scattered into the camera by the clouds and the atmosphere (global illumination)
            sun_color * sun_scatter * get_sun_brightness() * (sun_access * (1.0 - cloud_darken) * cloud_diffuse /*+ sky_color * global_scatter_factor*/) +
            moon_color * moon_scatter * get_moon_brightness() * (moon_access * (1.0 - cloud_darken) * cloud_diffuse /*+ sky_color * global_scatter_factor*/) +
            sky_light * (1.0 - global_darken) * not_underground +
            // A small amount fake ambient light underground
            (1.0 - not_underground) * vec3(0.2, 0.35, 0.5) * (1.0 - global_darken) / (1.0 + max_dist * 0.003) +
            emission * density_integrals.y * step;

        // Rainbow
        #if (CLOUD_MODE >= CLOUD_MODE_ULTRA)
        #ifndef EXPERIMENTAL_NORAINBOWS
            if (rainbow_c >= 0 && rainbow_c < 8) {
                vec3 colors[9] = {
                    surf_color,
                    vec3(0.9, 0.5, 0.9),
                    vec3(0.25, 0.0, 0.5),
                    vec3(0.0, 0.0, 1.0),
                    vec3(0.0, 0.5, 0.0),
                    vec3(1.0, 1.0, 0.0),
                    vec3(1.0, 0.6, 0.0),
                    vec3(1.0, 0.0, 0.0),
                    surf_color,
                };
                float h = max(0.0, min(pos.z, 900.0 - pos.z) / 450.0);
                float rain = rain_density_at(pos.xy) * pow(h, 0.1);

                float sun = sun_access * get_sun_brightness();
                float energy = pow(rain * sun * min(cdist / 500.0, 1.0), 2.0) * 0.4;

                surf_color = mix(
                    surf_color,
                    mix(colors[rainbow_c], colors[rainbow_c + 1], rainbow_t),
                    energy
                );
            }
        #endif
        #endif
    }

    // Underwater light attenuation
    surf_color = water_diffuse(surf_color, dir, max_dist);

    // Apply point glow
    surf_color = apply_point_glow(origin, dir, max_dist, surf_color);

    return surf_color;
}
