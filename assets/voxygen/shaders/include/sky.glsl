#include <random.glsl>

uniform sampler2D t_noise;

const float PI = 3.141592;

const vec3 SKY_DAY_TOP = vec3(0.1, 0.2, 0.9);
const vec3 SKY_DAY_MID = vec3(0.02, 0.08, 0.8);
const vec3 SKY_DAY_BOT = vec3(0.1, 0.2, 0.3);
const vec3 DAY_LIGHT   = vec3(1.2, 1.0, 1.0);
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

const float PERSISTENT_AMBIANCE = 0.1;

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

void get_sun_diffuse(vec3 norm, float time_of_day, out vec3 light, out vec3 diffuse_light, out vec3 ambient_light, float diffusion) {
	const float SUN_AMBIANCE = 0.1;

	vec3 sun_dir = get_sun_dir(time_of_day);
	vec3 moon_dir = get_moon_dir(time_of_day);

	float sun_light = get_sun_brightness(sun_dir);
	float moon_light = get_moon_brightness(moon_dir);

	// clamp() changed to max() as sun_dir.z is produced from a cos() function and therefore never greater than 1

	vec3 sun_color = get_sun_color(sun_dir);

	vec3 moon_color = get_moon_color(moon_dir);

	vec3 sun_chroma = sun_color * sun_light;
	vec3 moon_chroma = moon_color * moon_light;

	light = sun_chroma + moon_chroma + PERSISTENT_AMBIANCE;
	diffuse_light =
		sun_chroma * mix(1.0, max(dot(-norm, sun_dir) * 0.6 + 0.4, 0.0), diffusion) +
		moon_chroma * mix(1.0, pow(max(dot(-norm, moon_dir), 0.0), 2.0), diffusion) +
		PERSISTENT_AMBIANCE;
	ambient_light = vec3(SUN_AMBIANCE * sun_light + moon_light);
}

// This has been extracted into a function to allow quick exit when detecting a star.
float is_star_at(vec3 dir) {
	float star_scale = 30.0;

	for (int i = 0; i < 2; i ++) {
		for (int j = 0; j < 2; j ++) {
			for (int k = 0; k < 2; k ++) {
				// Star positions
				vec3 pos = (floor(dir * star_scale) + vec3(i, j, k) - vec3(0.5)) / star_scale;

				// Noisy offsets
				pos += (3.0 / star_scale) * rand_perm_3(pos);

				// Find distance to fragment
				float dist = length(normalize(pos) - dir);

				// Star threshold
				if (dist < 0.0015) {
					return 1.0;
				}
			}
		}
	}

	return 0.0;
}

const float CLOUD_AVG_HEIGHT = 1025.0;
const float CLOUD_HEIGHT_MIN = CLOUD_AVG_HEIGHT - 30.0;
const float CLOUD_HEIGHT_MAX = CLOUD_AVG_HEIGHT + 30.0;
const float CLOUD_THRESHOLD = 0.3;
const float CLOUD_SCALE = 1.0;
const float CLOUD_DENSITY = 100.0;

float vsum(vec3 v) {
	return v.x + v.y + v.z;
}

vec2 cloud_at(vec3 pos) {
	float tick_offs = 0.0
		+ texture(t_noise, pos.xy * 0.0001 + tick.x * 0.001).x * 1.0
		+ texture(t_noise, pos.xy * 0.000003).x * 5.0;

	float value = (
		0.0
		+ texture(t_noise, pos.xy / CLOUD_SCALE * 0.0003 + tick_offs).x
		+ texture(t_noise, pos.xy / CLOUD_SCALE * 0.0009 - tick_offs).x * 0.5
		+ texture(t_noise, pos.xy / CLOUD_SCALE * 0.0025 - tick.x * 0.01).x * 0.25
		+ texture(t_noise, pos.xy / CLOUD_SCALE * 0.008 + tick.x * 0.02).x * 0.1
	) / 3.0;

	float density = max((value - CLOUD_THRESHOLD) - abs(pos.z - CLOUD_AVG_HEIGHT) / 500.0, 0.0) * CLOUD_DENSITY;

	float shade = 1.0 - min(pow(max(CLOUD_AVG_HEIGHT - pos.z, 0.0), 0.15) * 0.5, 1.0) / 0.5;

	return vec2(shade, density / (1.0 + vsum(abs(pos - cam_pos.xyz)) / 5000));
}

vec4 get_cloud_color(vec3 dir, vec3 origin, float time_of_day, float max_dist, float quality) {
	const float INCR = 0.07;

	float mind = (CLOUD_HEIGHT_MIN - origin.z) / dir.z;
	float maxd = (CLOUD_HEIGHT_MAX - origin.z) / dir.z;

	float start = max(min(mind, maxd), 0.0);
	float delta = min(abs(mind - maxd), 5000.0);
	delta = min(delta, max_dist);

	bool do_cast = true;
	if (mind < 0.0 && maxd < 0.0) {
		do_cast = false;
	}

	float incr = clamp(INCR / (0.01 * delta * quality), INCR, INCR * 3.0);

	float fuzz = sin(texture(t_noise, dir.xz * 100000.0).x * 100.0) * 1.0 * incr * delta;

	float cloud_shade = 1.0;
	float passthrough = 1.0;
	if (do_cast) {
		for (float d = 0.0; d < 1.0; d += incr) {
			float dist = start + d * delta;
			dist += fuzz * min(pow(dist * 0.005, 2.0), 1.0);

			vec3 pos = origin + dir * min(dist, max_dist);
			vec2 sample = cloud_at(pos);

			float integral = sample.y * incr;
			passthrough *= 1.0 - integral;
			cloud_shade = mix(cloud_shade, sample.x, passthrough * integral);
		}
	}

	float total_density = 1.0 - passthrough / (1.0 + min(delta, max_dist) * 0.0003);

	total_density = max(total_density - 1.0 / pow(max_dist, 0.25), 0.0); // Hack

	return vec4(vec3(cloud_shade), total_density);
}

vec3 get_sky_color(vec3 dir, float time_of_day, vec3 origin, vec3 f_pos, float quality, bool with_stars, out vec4 clouds) {
	// Sky color
	vec3 sun_dir = get_sun_dir(time_of_day);
	vec3 moon_dir = get_moon_dir(time_of_day);

	// Add white dots for stars. Note these flicker and jump due to FXAA
	float star = 0.0;
	if (with_stars) {
		star = is_star_at(dir);
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
	vec3 moon_light = clamp(moon_halo + moon_surf, vec3(0), vec3(clamp(dir.z * 3.0, 0, 1)));

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
	clouds.rgb *= get_sun_brightness(sun_dir) * (sun_halo * 2.5 + get_sun_color(sun_dir)) + get_moon_brightness(moon_dir) * (moon_halo * 80.0 + get_moon_color(moon_dir));

	if (f_dist > 5000.0) {
		sky_color += sun_light + moon_light;
	}

	return mix(sky_color, clouds.rgb, clouds.a);
}

float fog(vec3 f_pos, vec3 focus_pos, uint medium) {
	float fog_radius = view_distance.x;
	float mist_radius = 10000000.0;

	float min_fog = 0.5;
	float max_fog = 1.0;

	if (medium == 1u) {
		mist_radius = 96.0;
		min_fog = 0.0;
	}

	float fog = distance(f_pos.xy, focus_pos.xy) / fog_radius;
	float mist = distance(f_pos, focus_pos) / mist_radius;

	return pow(clamp((max(fog, mist) - min_fog) / (max_fog - min_fog), 0.0, 1.0), 1.7);
}
