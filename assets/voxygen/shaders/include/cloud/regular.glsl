#include <random.glsl>
#include <lod.glsl>

const float CLOUD_THRESHOLD = 0.27;
const float CLOUD_SCALE = 5.0;
const float CLOUD_DENSITY = 150.0;

vec2 get_cloud_heights(vec2 pos) {
    const float CLOUD_HALF_WIDTH = 300;
    const float CLOUD_HEIGHT_VARIATION = 1000.0;
    float cloud_alt = CLOUD_AVG_ALT + (texture(t_noise, pos.xy * 0.0001).x - 0.5) * CLOUD_HEIGHT_VARIATION;
    #if (CLOUD_MODE != CLOUD_MODE_MINIMAL)
        cloud_alt += (texture(t_noise, pos.xy * 0.001).x - 0.5) * 0.1 * CLOUD_HEIGHT_VARIATION;
    #endif
    return vec2(cloud_alt, CLOUD_HALF_WIDTH);
}

// Returns vec4(r, g, b, density)
vec3 cloud_at(vec3 pos, float dist) {
    // Natural attenuation of air (air naturally attenuates light that passes through it)
    // Simulate the atmosphere thinning above 3000 metres down to nothing at 5000 metres
    float air = 0.00005 * clamp((3000.0 - pos.z) / 2000, 0, 1);

    // Mist sits close to the ground in valleys (TODO: use base_alt to put it closer to water)
    float MIST_MIN = 300;
    const float MIST_FADE_HEIGHT = 250;
    float mist = 0.00025 * pow(clamp(1.0 - (pos.z - MIST_MIN) / MIST_FADE_HEIGHT, 0.0, 1), 2) / (1.0 + pow(1.0 + dist / 20000.0, 2.0));

    vec3 wind_pos = vec3(pos.xy + wind_offset, pos.z);

    // Clouds
    float cloud_tendency = cloud_tendency_at(pos.xy);
    float sun_access = 0.05;
    float cloud = 0;

    vec2 cloud_attr = get_cloud_heights(wind_pos.xy);
    float cloud_factor = 0.0;
    // This is a silly optimisation but it actually nets us a fair few fps by skipping quite a few expensive calcs
    if (cloud_tendency > 0 || mist > 0.0) {
        // Turbulence (small variations in clouds/mist)
        const float turb_speed = -1.0; // Turbulence goes the opposite way
        vec3 turb_offset = vec3(1, 1, 0) * time_of_day.x * turb_speed;
        #if (CLOUD_MODE == CLOUD_MODE_MINIMAL)
            float turb_noise = 0.0;
        #else
            float turb_noise = noise_3d((wind_pos + turb_offset) * 0.001) - 0.5;
        #endif
        #if (CLOUD_MODE == CLOUD_MODE_MEDIUM || CLOUD_MODE == CLOUD_MODE_HIGH)
            turb_noise += (noise_3d((wind_pos + turb_offset * 0.3) * 0.004) - 0.5) * 0.25;
        #endif
        mist *= (1.0 + turb_noise);

        cloud_factor = 0.25 * (1.0 - pow(min(abs(pos.z - cloud_attr.x) / (cloud_attr.y * pow(max(cloud_tendency * 20.0, 0), 0.5)), 1.0), 2.0));
        float cloud_flat = min(cloud_tendency, 0.07) * 0.05;
        cloud_flat *= (1.0 + turb_noise * 7.0 * max(0, 1.0 - cloud_factor * 5));
        cloud = cloud_flat * pow(cloud_factor, 2) * 20 / (1 + pow(1.0 + dist / 10000.0, 2.0));
    }

    // What proportion of sunlight is *not* being blocked by nearby cloud? (approximation)
    sun_access = clamp((pos.z - cloud_attr.x) * 0.002 + 0.35 + mist * 10000, 0.0, 1);

    // Prevent clouds and mist appearing underground (but fade them out gently)
    float not_underground = clamp(1.0 - (alt_at(pos.xy - focus_off.xy) - (pos.z - focus_off.z)) / 80.0, 0, 1);
    float vapor_density = (mist + cloud) * not_underground;

    // We track vapor density and air density separately. Why? Because photons will ionize particles in air
    // leading to rayleigh scattering, but water vapor will not. Tracking these indepedently allows us to
    // get more correct colours.
    return vec3(sun_access, vapor_density, air);
}

float atan2(in float y, in float x) {
    bool s = (abs(x) > abs(y));
    return mix(PI/2.0 - atan(x,y), atan(y,x), s);
}

const float DIST_CAP = 50000;
#if (CLOUD_MODE == CLOUD_MODE_HIGH)
    const uint QUALITY = 100u;
#elif (CLOUD_MODE == CLOUD_MODE_MEDIUM)
    const uint QUALITY = 40u;
#elif (CLOUD_MODE == CLOUD_MODE_LOW)
    const uint QUALITY = 20u;
#elif (CLOUD_MODE == CLOUD_MODE_MINIMAL)
    const uint QUALITY = 7u;
#endif

const float STEP_SCALE = DIST_CAP / (10.0 * float(QUALITY));

float step_to_dist(float step) {
    return pow(step, 2) * STEP_SCALE;
}

float dist_to_step(float dist) {
    return pow(dist / STEP_SCALE, 0.5);
}

vec3 get_cloud_color(vec3 surf_color, vec3 dir, vec3 origin, const float time_of_day, float max_dist, const float quality) {
    // Limit the marching distance to reduce maximum jumps
    max_dist = min(max_dist, DIST_CAP);

    origin.xyz += focus_off.xyz;

    // This hack adds a little direction-dependent noise to clouds. It's not correct, but it very cheaply
    // improves visual quality for low cloud settings
    float splay = 1.0;
    vec3 dir_diff = vec3(0);
    #if (CLOUD_MODE == CLOUD_MODE_MINIMAL)
        /* splay += (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 1.5 - time_of_day * 0.000025).x - 0.5) * 0.4 / (1.0 + pow(dir.z, 2) * 10); */
        dir_diff = vec3(
            (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 1.0 - time_of_day * 0.00005).x - 0.5) * 0.2 / (1.0 + pow(dir.z, 2) * 10),
            (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 1.0 - time_of_day * 0.00005).x - 0.5) * 0.2 / (1.0 + pow(dir.z, 2) * 10),
            (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 1.0 - time_of_day * 0.00005).x - 0.5) * 0.2 / (1.0 + pow(dir.z, 2) * 10)
        ) * 2000;
    #endif
    #if (CLOUD_MODE == CLOUD_MODE_MINIMAL || CLOUD_MODE == CLOUD_MODE_LOW)
        splay += (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 10.0 - time_of_day * 0.00005).x - 0.5) * 0.075 / (1.0 + pow(dir.z, 2) * 10);
    #endif

    // Proportion of sunlight that get scattered back into the camera by clouds
    float sun_scatter = max(dot(-dir, sun_dir.xyz), 0.5);
    float moon_scatter = max(dot(-dir, moon_dir.xyz), 0.5);
    vec3 sky_color = get_sky_color();
    vec3 directed_scatter =
        // Sun scatter
        get_sun_color() * get_sun_brightness() * sun_scatter +
        // Moon scatter
        get_moon_color() * get_moon_brightness() * moon_scatter;

    float cdist = max_dist;
    while (cdist > 1) {
        float ndist = step_to_dist(trunc(dist_to_step(cdist - 0.25)));
        vec3 sample = cloud_at(origin + (dir + dir_diff / ndist) * ndist * splay, ndist);

        vec2 density_integrals = sample.yz * (cdist - ndist);

        float sun_access = sample.x;
        float scatter_factor = 1.0 - 1.0 / (1.0 + density_integrals.x);

        surf_color =
            // Attenuate light passing through the clouds, removing light due to rayleigh scattering (transmission component)
            surf_color * (1.0 - scatter_factor) - surf_color * density_integrals.y * sky_color +
            // This is not rayleigh scattering, but it's good enough for our purposes
            sky_color * density_integrals.y +
            // Add the directed light light scattered into the camera by the clouds
            directed_scatter * sun_access * scatter_factor +
            // Global illumination (uniform scatter from the sky)
            sky_color * sun_access * scatter_factor;

        cdist = ndist;
    }

    return surf_color;
}
