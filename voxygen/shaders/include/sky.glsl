const float PI = 3.141592;

const vec3 SKY_DAY_TOP = vec3(0.35, 0.45, 0.9);
const vec3 SKY_DAY_MID = vec3(0.25, 0.35, 0.8);
const vec3 SKY_DAY_BOT = vec3(0.02, 0.1, 0.3);
const vec3 DAY_LIGHT   = vec3(0.5, 0.5, 1.0);

const vec3 SKY_DUSK_TOP = vec3(0.1, 0.15, 0.3);
const vec3 SKY_DUSK_MID = vec3(0.95, 0.4, 0.25);
const vec3 SKY_DUSK_BOT = vec3(0.01, 0.05, 0.15);
const vec3 DUSK_LIGHT   = vec3(0.9, 0.2, 0.1);

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
	return max(-sun_dir.z + 0.6, 0.0);
}

const float PERSISTENT_AMBIANCE = 0.008;

vec3 get_sun_diffuse(vec3 norm, float time_of_day) {
	const float SUN_AMBIANCE = 0.075;

	vec3 sun_dir = get_sun_dir(time_of_day);

	float sun_light = get_sun_brightness(sun_dir);

	vec3 sun_color = normalize(mix(
		mix(
			DUSK_LIGHT,
			NIGHT_LIGHT,
			clamp(sun_dir.z, 0, 1)
		),
		DAY_LIGHT,
		clamp(-sun_dir.z, 0, 1)
	));

	vec3 diffuse_light = (SUN_AMBIANCE + max(dot(-norm, sun_dir), 0.0) * sun_color) * sun_light + PERSISTENT_AMBIANCE;

	return diffuse_light;
}

vec3 rand_offs(vec3 pos) {
	return sin(pos * vec3(1473.7 * pos.z + 472.3, 8891.1 * pos.x + 723.1, 3813.3 * pos.y + 982.5));
}

vec3 get_sky_color(vec3 dir, float time_of_day) {

	// Stars
	float star_scale = 30.0;
	float star = 0.0;
	for (int i = 0; i < 2; i ++) {
		for (int j = 0; j < 2; j ++) {
			for (int k = 0; k < 2; k ++) {
				// Star positions
				vec3 pos = (floor(dir * star_scale) + vec3(i, j, k) - vec3(0.5)) / star_scale;

				// Noisy offsets
				pos += 3.0 * rand_offs(pos) / star_scale;

				// Find distance to fragment
				float dist = length(normalize(pos) - dir);

				// Star threshold
				if (dist < 0.0015) {
					star = 1.0;
				}
			}
		}
	}

	// Sky color

	vec3 sun_dir = get_sun_dir(time_of_day);

	vec3 sky_top = mix(
		mix(
			SKY_DUSK_TOP,
			SKY_NIGHT_TOP,
			clamp(sun_dir.z, 0, 1)
		) + star,
		SKY_DAY_TOP,
		clamp(-sun_dir.z, 0, 1)
	);

	vec3 sky_mid = mix(
		mix(
			SKY_DUSK_MID,
			SKY_NIGHT_MID,
			clamp(sun_dir.z, 0, 1)
		),
		SKY_DAY_MID,
		clamp(-sun_dir.z, 0, 1)
	);

	vec3 sky_bot = mix(
		mix(
			SKY_DUSK_BOT,
			SKY_NIGHT_BOT,
			clamp(sun_dir.z, 0, 1)
		),
		SKY_DAY_BOT,
		clamp(-sun_dir.z, 0, 1)
	);

	vec3 sky_color = mix(
		mix(
			sky_mid,
			sky_bot,
			pow(clamp(-dir.z, 0, 1), 0.4)
		),
		sky_top,
		pow(clamp(dir.z, 0, 1), 1.0)
	);

	// Sun

	const vec3 SUN_HALO_COLOR = vec3(1.0, 0.35, 0.1) * 0.3;
	const vec3 SUN_SURF_COLOR = vec3(1.0, 0.9, 0.35) * 200.0;

	vec3 sun_halo = pow(max(dot(dir, -sun_dir) + 0.1, 0.0), 8.0) * SUN_HALO_COLOR;
	vec3 sun_surf = pow(max(dot(dir, -sun_dir) - 0.0045, 0.0), 1000.0) * SUN_SURF_COLOR;
	vec3 sun_light = (sun_halo + sun_surf) * clamp(dir.z * 10.0, 0, 1);

	return sky_color + sun_light;
}

float fog(vec2 f_pos, vec2 focus_pos) {
	float dist = distance(f_pos, focus_pos) / view_distance.x;
	float min_fog = 0.75;
	float max_fog = 1.0;

	return clamp((dist - min_fog) / (max_fog - min_fog), 0.0, 1.0);
}
