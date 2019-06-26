const float PI = 3.141592;

float mod289(float x){return x - floor(x * (1.0 / 289.0)) * 289.0;}
vec4 mod289(vec4 x){return x - floor(x * (1.0 / 289.0)) * 289.0;}
vec4 perm(vec4 x){return mod289(((x * 34.0) + 1.0) * x);}

float noise(vec3 p){
    vec3 a = floor(p);
    vec3 d = p - a;
    d = d * d * (3.0 - 2.0 * d);

    vec4 b = a.xxyy + vec4(0.0, 1.0, 0.0, 1.0);
    vec4 k1 = perm(b.xyxy);
    vec4 k2 = perm(k1.xyxy + b.zzww);

    vec4 c = k2 + a.zzzz;
    vec4 k3 = perm(c);
    vec4 k4 = perm(c + 1.0);

    vec4 o1 = fract(k3 * (1.0 / 41.0));
    vec4 o2 = fract(k4 * (1.0 / 41.0));

    vec4 o3 = o2 * d.z + o1 * (1.0 - d.z);
    vec2 o4 = o3.yw * d.x + o3.xz * (1.0 - d.x);

    return o4.y * d.y + o4.x * (1.0 - d.y);
}

vec3 get_sun_dir(float time_of_day) {
	const float TIME_FACTOR = (PI * 2.0) / (3600.0 * 24.0);

	float sun_angle_rad = time_of_day * TIME_FACTOR;
	vec3 sun_dir = vec3(sin(sun_angle_rad), 0.0, cos(sun_angle_rad));

	return sun_dir;
}

float get_sun_brightness(vec3 sun_dir) {
	return max(-sun_dir.z + 0.6, 0.0) * 0.8;
}

const float PERSISTENT_AMBIANCE = 0.015;

float get_sun_diffuse(vec3 norm, float time_of_day) {
	const float SUN_AMBIANCE = 0.2;

	vec3 sun_dir = get_sun_dir(time_of_day);

	float sun_light = get_sun_brightness(sun_dir);

	return (SUN_AMBIANCE + max(dot(-norm, sun_dir), 0.0)) * sun_light + PERSISTENT_AMBIANCE;
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

	const vec3 SKY_DAY_TOP = vec3(0.2, 0.3, 0.9);
	const vec3 SKY_DAY_MID = vec3(0.1, 0.15, 0.7);
	const vec3 SKY_DAY_BOT = vec3(0.025, 0.15, 0.35);

	const vec3 SKY_DUSK_TOP = vec3(0.1, 0.15, 0.3);
	const vec3 SKY_DUSK_MID = vec3(0.9, 0.3, 0.2);
	const vec3 SKY_DUSK_BOT = vec3(0.01, 0.05, 0.15);

	const vec3 SKY_NIGHT_TOP = vec3(0.002, 0.002, 0.005);
	const vec3 SKY_NIGHT_MID = vec3(0.002, 0.01, 0.03);
	const vec3 SKY_NIGHT_BOT = vec3(0.002, 0.002, 0.005);

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
		SKY_DAY_MID,
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












	/*
	bool objects = true;

	vec2 pos2d = dir.xy / dir.z;

	const vec3 SKY_TOP    = vec3(0.2, 0.3, 0.9);
	const vec3 SKY_MIDDLE = vec3(0.1, 0.15, 0.7);
	const vec3 SKY_BOTTOM = vec3(0.025, 0.15, 0.35);

	const vec3 SUN_HALO_COLOR = vec3(1.0, 0.4, 0.3) * 0.5;
	const vec3 SUN_SURF_COLOR = vec3(1.0, 0.9, 0.35) * 200.0;

	vec3 sun_dir = get_sun_dir(time_of_day);
	float sky_brightness = get_sun_brightness(sun_dir);

	vec3 sun_halo = pow(max(dot(dir, -sun_dir) + 0.1, 0.0), 8.0) * SUN_HALO_COLOR;
	vec3 sun_surf = pow(max(dot(dir, -sun_dir) - 0.0045, 0.0), 1000.0) * SUN_SURF_COLOR;
	vec3 sun_light = sun_halo + sun_surf;

	float brightess = (sky_brightness + PERSISTENT_AMBIANCE);

	vec3 sky_top = SKY_TOP * brightess;
	vec3 sky_middle = SKY_MIDDLE * brightess;
	if (objects) {
		// Clouds
		// vec3 p = vec3(pos2d + time_of_day * 0.0002, time_of_day * 0.00003);
		// sky_top = mix(sky_top, vec3(1) * brightess, pow(noise(p) * 0.8 + noise(p * 3.0) * 0.2, 2.5) * 3.0);
	}

	if (objects) {
		sky_top += sun_light;
		sky_middle += sun_light;
	}

	vec3 sky_color = mix(
		mix(
			sky_middle,
			sky_top,
			clamp(dir.z * 1.0, 0, 1)
		),
		SKY_BOTTOM * brightess,
		clamp(-dir.z * 3.0, 0, 1)
	);

	return sky_color;
	*/
}

float fog(vec2 f_pos, vec2 focus_pos) {
	float dist = distance(f_pos, focus_pos) / view_distance.x;
	float min_fog = 0.75;
	float max_fog = 1.0;

	return clamp((dist - min_fog) / (max_fog - min_fog), 0.0, 1.0);
}
