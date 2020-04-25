#include <random.glsl>
#include <srgb.glsl>
#include <cloud.glsl>

const float PI = 3.141592;

const vec3 SKY_DAY_TOP = vec3(0.1, 0.5, 0.9);
const vec3 SKY_DAY_MID = vec3(0.02, 0.28, 0.8);
const vec3 SKY_DAY_BOT = vec3(0.1, 0.2, 0.3);
const vec3 DAY_LIGHT   = vec3(1.2, 1.0, 1.0) * 3.0;
const vec3 SUN_HALO_DAY = vec3(0.35, 0.35, 0.0);

const vec3 SKY_DUSK_TOP = vec3(0.06, 0.1, 0.20);
const vec3 SKY_DUSK_MID = vec3(0.35, 0.1, 0.15);
const vec3 SKY_DUSK_BOT = vec3(0.0, 0.1, 0.23);
const vec3 DUSK_LIGHT   = vec3(3.0, 1.5, 0.3);
const vec3 SUN_HALO_DUSK = vec3(1.2, 0.15, 0.0);

const vec3 SKY_NIGHT_TOP = vec3(0.001, 0.001, 0.0025);
const vec3 SKY_NIGHT_MID = vec3(0.001, 0.005, 0.02);
const vec3 SKY_NIGHT_BOT = vec3(0.002, 0.004, 0.004);
const vec3 NIGHT_LIGHT   = vec3(0.002, 0.01, 0.03);

const float UNDERWATER_MIST_DIST = 100.0;

vec3 get_sun_dir(float time_of_day) {
	const float TIME_FACTOR = (PI * 2.0) / (3600.0 * 24.0);

	float sun_angle_rad = time_of_day * TIME_FACTOR;
	return vec3(sin(sun_angle_rad), 0.0, cos(sun_angle_rad));
}

vec3 get_moon_dir(float time_of_day) {
	const float TIME_FACTOR = (PI * 2.0) / (3600.0 * 24.0);

	float moon_angle_rad = time_of_day * TIME_FACTOR;
	return normalize(-vec3(sin(moon_angle_rad), 0.0, cos(moon_angle_rad) - 0.5));
}

const float PERSISTENT_AMBIANCE = 0.0125; // 0.1;// 0.025; // 0.1;

float get_sun_brightness(vec3 sun_dir) {
	return max(-sun_dir.z + 0.6, 0.0) * 0.9;
}

float get_moon_brightness(vec3 moon_dir) {
	return max(-moon_dir.z + 0.6, 0.0) * 0.07;
}

vec3 get_sun_color(vec3 sun_dir) {
	return mix(
		mix(
			DUSK_LIGHT,
			NIGHT_LIGHT,
			max(sun_dir.z, 0)
		),
		DAY_LIGHT,
		max(-sun_dir.z, 0)
	);
}

vec3 get_moon_color(vec3 moon_dir) {
	return vec3(0.05, 0.05, 0.6);
}

// Calculates extra emission and reflectance (due to sunlight / moonlight).
//
// reflectence = k_a * i_a + i_a,persistent
// emittence = Σ { m ∈ lights } i_m * shadow_m * get_light_reflected(light_m)
//
// Note that any shadowing to be done that would block the sun and moon, aside from heightmap shadowing (that will be
// implemented sooon), should be implicitly provided via k_a, k_d, and k_s.  For instance, shadowing via ambient occlusion.
//
// Also note that the emitted light calculation is kind of lame... we probabbly need something a bit nicer if we ever want to do
// anything interesting here.
// void get_sun_diffuse(vec3 norm, float time_of_day, out vec3 light, out vec3 diffuse_light, out vec3 ambient_light, float diffusion
void get_sun_diffuse(vec3 norm, float time_of_day, vec3 dir, vec3 k_a, vec3 k_d, vec3 k_s, float alpha, out vec3 emitted_light, out vec3 reflected_light) {
	const float SUN_AMBIANCE = 0.1 / 2.0;// 0.1 / 3.0;

	vec3 sun_dir = get_sun_dir(time_of_day);
	vec3 moon_dir = get_moon_dir(time_of_day);

	float sun_light = get_sun_brightness(sun_dir);
	float moon_light = get_moon_brightness(moon_dir);

	vec3 sun_color = get_sun_color(sun_dir);
	vec3 moon_color = get_moon_color(moon_dir);

	vec3 sun_chroma = sun_color * sun_light;
	vec3 moon_chroma = moon_color * moon_light;

    /* float NLsun = max(dot(-norm, sun_dir), 0);
    float NLmoon = max(dot(-norm, moon_dir), 0);
    vec3 E = -dir; */

    // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-norm, sun_dir)) * mix(0.0, 1.0, abs(sun_dir.z) * 10000.0) * 10000.0), 0.0, 0.5);
    // float ambient_sides = 0.5 - 0.5 * abs(dot(-norm, sun_dir));

    emitted_light = k_a * (ambient_sides + vec3(SUN_AMBIANCE * sun_light + moon_light)) + PERSISTENT_AMBIANCE;
    // TODO: Add shadows.
    reflected_light =
        sun_chroma * light_reflection_factor(norm, dir, sun_dir, k_d, k_s, alpha) +
        moon_chroma * 1.0 * /*4.0 * */light_reflection_factor(norm, dir, moon_dir, k_d, k_s, alpha);

	/* light = sun_chroma + moon_chroma + PERSISTENT_AMBIANCE;
	diffuse_light =
		sun_chroma * mix(1.0, max(dot(-norm, sun_dir) * 0.5 + 0.5, 0.0), diffusion) +
		moon_chroma * mix(1.0, pow(dot(-norm, moon_dir) * 2.0, 2.0), diffusion) +
		PERSISTENT_AMBIANCE;
	ambient_light = vec3(SUN_AMBIANCE * sun_light + moon_light); */
}

// Returns computed maximum intensity.
float get_sun_diffuse2(vec3 norm, vec3 sun_dir, vec3 moon_dir, vec3 dir, vec3 k_a, vec3 k_d, vec3 k_s, float alpha, out vec3 emitted_light, out vec3 reflected_light) {
	const float SUN_AMBIANCE = 0.23 / 1.8;// 0.1 / 3.0;
	const float MOON_AMBIANCE = 0.23;//0.1;

	float sun_light = get_sun_brightness(sun_dir);
	float moon_light = get_moon_brightness(moon_dir);

	vec3 sun_color = get_sun_color(sun_dir);
	vec3 moon_color = get_moon_color(moon_dir);

	vec3 sun_chroma = sun_color * sun_light;
	vec3 moon_chroma = moon_color * moon_light;

    // https://en.m.wikipedia.org/wiki/Diffuse_sky_radiation
    //
    // HdRd radiation should come in at angle normal to us.
    // const float H_d = 0.23;
    // Assuming we are on the equator:
    // R_b = (cos(h)cos(-β) / cos(h)) = cos(-β), the angle from horizontal.
    // NOTE: cos(-β) = cos(β).
    float cos_sun = dot(norm, -sun_dir);
    float cos_moon = dot(norm, -moon_dir);
    vec3 light_frac = /*vec3(1.0)*//*H_d * */
        SUN_AMBIANCE * /*sun_light*/sun_chroma * light_reflection_factor(norm, dir, /*vec3(0, 0, -1.0)*/-norm, vec3((1.0 + cos_sun) * 0.5), vec3(k_s * (1.0 - cos_sun) * 0.5), alpha) +
        MOON_AMBIANCE * /*sun_light*/moon_chroma * light_reflection_factor(norm, dir, /*vec3(0, 0, -1.0)*/-norm, vec3((1.0 + cos_moon) * 0.5), vec3(k_s * (1.0 - cos_moon) * 0.5), alpha);
    /* float NLsun = max(dot(-norm, sun_dir), 0);
    float NLmoon = max(dot(-norm, moon_dir), 0);
    vec3 E = -dir; */

    // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    // float ambient_sides = 0.0;
    // float ambient_sides = 0.5 - 0.5 * min(abs(dot(-norm, sun_dir)), abs(dot(-norm, moon_dir)));
    // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-norm, sun_dir)) * mix(0.0, 1.0, abs(sun_dir.z) * 10000.0) * 10000.0), 0.0, 0.5);
    // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-norm, sun_dir)) * mix(0.0, 1.0, abs(sun_dir.z) * 10000.0) * 10000.0), 0.0, 0.5);


    emitted_light = k_a * light_frac * (/*ambient_sides + */SUN_AMBIANCE * /*sun_light*/sun_chroma + /*vec3(moon_light)*/MOON_AMBIANCE * moon_chroma) + PERSISTENT_AMBIANCE;

    // TODO: Add shadows.
    reflected_light =
        (1.0 - SUN_AMBIANCE) * sun_chroma * (light_reflection_factor(norm, dir, sun_dir, k_d, k_s, alpha) /*+
                      light_reflection_factor(norm, dir, normalize(sun_dir + vec3(0.0, 0.1, 0.0)), k_d, k_s, alpha) +
                      light_reflection_factor(norm, dir, normalize(sun_dir - vec3(0.0, 0.1, 0.0)), k_d, k_s, alpha)*/) +
        (1.0 - MOON_AMBIANCE) * moon_chroma * 1.0 * /*4.0 * */light_reflection_factor(norm, dir, moon_dir, k_d, k_s, alpha);

	/* light = sun_chroma + moon_chroma + PERSISTENT_AMBIANCE;
	diffuse_light =
		sun_chroma * mix(1.0, max(dot(-norm, sun_dir) * 0.5 + 0.5, 0.0), diffusion) +
		moon_chroma * mix(1.0, pow(dot(-norm, moon_dir) * 2.0, 2.0), diffusion) +
		PERSISTENT_AMBIANCE;
	ambient_light = vec3(SUN_AMBIANCE * sun_light + moon_light); */
    return 1.0;//sun_chroma + moon_chroma + PERSISTENT_AMBIANCE;
}


// This has been extracted into a function to allow quick exit when detecting a star.
float is_star_at(vec3 dir) {
	float star_scale = 80.0;

	// Star positions
	vec3 pos = (floor(dir * star_scale) - 0.5) / star_scale;

	// Noisy offsets
	pos += (3.0 / star_scale) * rand_perm_3(pos);

	// Find distance to fragment
	float dist = length(normalize(pos) - dir);

	// Star threshold
	if (dist < 0.0015) {
		return 1.0;
	}

	return 0.0;
}

vec3 get_sky_color(vec3 dir, float time_of_day, vec3 origin, vec3 f_pos, float quality, bool with_stars, out vec4 clouds) {
	// Sky color
	vec3 sun_dir = get_sun_dir(time_of_day);
	vec3 moon_dir = get_moon_dir(time_of_day);

	// Add white dots for stars. Note these flicker and jump due to FXAA
	float star = 0.0;
	if (with_stars) {
		vec3 star_dir = normalize(sun_dir * dir.z + cross(sun_dir, vec3(0, 1, 0)) * dir.x + vec3(0, 1, 0) * dir.y);
		star = is_star_at(star_dir);
	}

	// Sun
	const vec3 SUN_SURF_COLOR = vec3(1.5, 0.9, 0.35) * 200.0;

	vec3 sun_halo_color = mix(
		SUN_HALO_DUSK,
		SUN_HALO_DAY,
		max(-sun_dir.z, 0)
	);

	vec3 sun_halo = pow(max(dot(dir, -sun_dir) + 0.1, 0.0), 8.0) * sun_halo_color;
	vec3 sun_surf = pow(max(dot(dir, -sun_dir) - 0.001, 0.0), 3000.0) * SUN_SURF_COLOR;
	vec3 sun_light = (sun_halo + sun_surf) * clamp(dir.z * 10.0, 0, 1);

	// Moon
	const vec3 MOON_SURF_COLOR = vec3(0.7, 1.0, 1.5) * 500.0;
	const vec3 MOON_HALO_COLOR = vec3(0.015, 0.015, 0.05);

	vec3 moon_halo = pow(max(dot(dir, -moon_dir) + 0.1, 0.0), 8.0) * MOON_HALO_COLOR;
	vec3 moon_surf = pow(max(dot(dir, -moon_dir) - 0.001, 0.0), 3000.0) * MOON_SURF_COLOR;
	vec3 moon_light = clamp(moon_halo + moon_surf, vec3(0), vec3(max(dir.z * 3.0, 0)));

	// Replaced all clamp(sun_dir, 0, 1) with max(sun_dir, 0) because sun_dir is calculated from sin and cos, which are never > 1

	vec3 sky_top = mix(
		mix(
			SKY_DUSK_TOP + star / (1.0 + moon_surf * 100.0),
			SKY_NIGHT_TOP + star / (1.0 + moon_surf * 100.0),
			max(pow(sun_dir.z, 0.2), 0)
		),
		SKY_DAY_TOP,
		max(-sun_dir.z, 0)
	);

	vec3 sky_mid = mix(
		mix(
			SKY_DUSK_MID,
			SKY_NIGHT_MID,
			max(pow(sun_dir.z, 0.2), 0)
		),
		SKY_DAY_MID,
		max(-sun_dir.z, 0)
	);

	vec3 sky_bot = mix(
		mix(
			SKY_DUSK_BOT,
			SKY_NIGHT_BOT,
			max(pow(sun_dir.z, 0.2), 0)
		),
		SKY_DAY_BOT,
		max(-sun_dir.z, 0)
	);

	vec3 sky_color = mix(
		mix(
			sky_mid,
			sky_bot,
			pow(max(-dir.z, 0), 0.4)
		),
		sky_top,
		max(dir.z, 0)
	);

	// Approximate distance to fragment
	float f_dist = distance(origin, f_pos);

	// Clouds
	clouds = get_cloud_color(dir, origin, time_of_day, f_dist, quality);
	clouds.rgb *= get_sun_brightness(sun_dir) * (sun_halo * 1.5 + get_sun_color(sun_dir)) + get_moon_brightness(moon_dir) * (moon_halo * 80.0 + get_moon_color(moon_dir) + 0.25);

	if (f_dist > 5000.0) {
		sky_color += sun_light + moon_light;
	}

	return mix(sky_color, clouds.rgb, clouds.a);
}

float fog(vec3 f_pos, vec3 focus_pos, uint medium) {
	return max(1.0 - 5000.0 / (1.0 + distance(f_pos.xy, focus_pos.xy)), 0.0);

	float fog_radius = view_distance.x;
	float mist_radius = 10000000.0;

	float min_fog = 0.5;
	float max_fog = 1.0;

	if (medium == 1u) {
		mist_radius = UNDERWATER_MIST_DIST;
		min_fog = 0.0;
	}

	float fog = distance(f_pos.xy, focus_pos.xy) / fog_radius;
	float mist = distance(f_pos, focus_pos) / mist_radius;

	return pow(clamp((max(fog, mist) - min_fog) / (max_fog - min_fog), 0.0, 1.0), 1.7);
}

float rel_luminance(vec3 rgb)
{
    // https://en.wikipedia.org/wiki/Relative_luminance
    const vec3 W = vec3(0.2126, 0.7152, 0.0722);
    return dot(rgb, W);
}

/* vec3 illuminate(vec3 color, vec3 light, vec3 diffuse, vec3 ambience) {
	float avg_col = (color.r + color.g + color.b) / 3.0;
	return ((color - avg_col) * light + (diffuse + ambience) * avg_col) * (diffuse + ambience);
} */
vec3 illuminate(/*vec3 max_light, */vec3 emitted, vec3 reflected) {
    const float gamma = /*0.5*//*1.*0*/1.0;//1.0;
    /* float light = length(emitted + reflected);
    float color = srgb_to_linear(emitted + reflected);
    float avg_col = (color.r + color.g + color.b) / 3.0;
    return ((color - avg_col) * light + reflected * avg_col) * (emitted + reflected); */
    // float max_intensity = vec3(1.0);
    vec3 color = emitted + reflected;
    float lum = rel_luminance(color);

    // Tone mapped value.
    // vec3 T = /*color*//*lum*/color;//normalize(color) * lum / (1.0 + lum);
    float alpha = 2.0;// 2.0;
    float T = 1.0 - exp(-alpha * lum);//lum / (1.0 + lum);
    // float T = lum;

    // Heuristic desaturation
    // float s = 0.5;
    vec3 col_adjusted = (color / lum);
    // vec3 c = pow(color / lum, vec3(s)) * T;
    // vec3 c = sqrt(col_adjusted) * T;
    vec3 c = col_adjusted * col_adjusted * T;

    return c;
    // float sum_col = color.r + color.g + color.b;
    // return /*srgb_to_linear*/(/*0.5*//*0.125 * */vec3(pow(color.x, gamma), pow(color.y, gamma), pow(color.z, gamma)));
}
