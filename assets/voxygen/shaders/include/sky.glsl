#include <random.glsl>

const float PI = 3.141592;

const vec3 SKY_DAY_TOP = vec3(0.1, 0.2, 0.9);
const vec3 SKY_DAY_MID = vec3(0.02, 0.08, 0.8);
const vec3 SKY_DAY_BOT = vec3(0.02, 0.01, 0.3);
const vec3 DAY_LIGHT   = vec3(1.2, 1.0, 1.0);

const vec3 SKY_DUSK_TOP = vec3(0.06, 0.1, 0.20);
const vec3 SKY_DUSK_MID = vec3(0.35, 0.1, 0.15);
const vec3 SKY_DUSK_BOT = vec3(0.0, 0.1, 0.13);
const vec3 DUSK_LIGHT   = vec3(3.0, 1.0, 0.3);

const vec3 SKY_NIGHT_TOP = vec3(0.001, 0.001, 0.0025);
const vec3 SKY_NIGHT_MID = vec3(0.001, 0.005, 0.02);
const vec3 SKY_NIGHT_BOT = vec3(0.002, 0.002, 0.005);
const vec3 NIGHT_LIGHT   = vec3(0.002, 0.01, 0.03);

vec3 get_sun_dir(float time_of_day) {
	const float TIME_FACTOR = (PI * 2.0) / (3600.0 * 24.0);

	float sun_angle_rad = time_of_day * TIME_FACTOR;
	vec3 sun_dir = vec3(sin(sun_angle_rad), 0.0, cos(sun_angle_rad));

	return sun_dir;
}

float get_sun_brightness(vec3 sun_dir) {
	return max(-sun_dir.z + 0.6, 0.0) * 1.0;
}

const float PERSISTENT_AMBIANCE = 0.008;

void get_sun_diffuse(vec3 norm, float time_of_day, out vec3 diffuse_light, out vec3 ambient_light) {
	const float SUN_AMBIANCE = 0.2;

	vec3 sun_dir = get_sun_dir(time_of_day);

	float sun_light = get_sun_brightness(sun_dir);

	// clamp() changed to max() as sun_dir.z is produced from a cos() function and therefore never greater than 1

	vec3 sun_color = mix(
		mix(
			DUSK_LIGHT,
			NIGHT_LIGHT,
			max(sun_dir.z, 0)
		),
		DAY_LIGHT,
		max(-sun_dir.z, 0)
	);

	diffuse_light = vec3(max(dot(-norm, sun_dir), 0.0) * sun_color * sun_light);
	ambient_light = vec3((SUN_AMBIANCE + PERSISTENT_AMBIANCE) * sun_light);
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

vec3 get_sky_color(vec3 dir, float time_of_day, bool with_stars) {
	// Sky color
	vec3 sun_dir = get_sun_dir(time_of_day);

	// Add white dots for stars. Note these flicker and jump due to FXAA
	float star = 0.0;
	if (with_stars) {
		star = is_star_at(dir);
	}

	// Replaced all clamp(sun_dir, 0, 1) with max(sun_dir, 0) because sun_dir is calculated from sin and cos, which are never > 1

	vec3 sky_top = mix(
		mix(
			SKY_DUSK_TOP + star,
			SKY_NIGHT_TOP + star,
			max(sun_dir.z, 0)
		),
		SKY_DAY_TOP,
		max(-sun_dir.z, 0)
	);

	vec3 sky_mid = mix(
		mix(
			SKY_DUSK_MID,
			SKY_NIGHT_MID,
			max(sun_dir.z, 0)
		),
		SKY_DAY_MID,
		max(-sun_dir.z, 0)
	);

	vec3 sky_bot = mix(
		mix(
			SKY_DUSK_BOT,
			SKY_NIGHT_BOT,
			max(sun_dir.z, 0)
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

	// Sun

	const vec3 SUN_HALO_COLOR = vec3(1.5, 0.65, 0.0) * 0.3;
	const vec3 SUN_SURF_COLOR = vec3(1.5, 0.9, 0.35) * 200.0;

	vec3 sun_halo = pow(max(dot(dir, -sun_dir) + 0.1, 0.0), 8.0) * SUN_HALO_COLOR;
	vec3 sun_surf = pow(max(dot(dir, -sun_dir) - 0.0045, 0.0), 1000.0) * SUN_SURF_COLOR;
	vec3 sun_light = (sun_halo + sun_surf) * clamp(dir.z * 10.0, 0, 1);

	return sky_color + sun_light;
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
