uniform sampler2D t_noise;

const float CLOUD_AVG_HEIGHT = 1025.0;
const float CLOUD_HEIGHT_MIN = CLOUD_AVG_HEIGHT - 60.0;
const float CLOUD_HEIGHT_MAX = CLOUD_AVG_HEIGHT + 60.0;
const float CLOUD_THRESHOLD = 0.27;
const float CLOUD_SCALE = 5.0;
const float CLOUD_DENSITY = 100.0;

float vsum(vec3 v) {
	return v.x + v.y + v.z;
}

vec2 cloud_at(vec3 pos) {
	vec2 scaled_pos = pos.xy / CLOUD_SCALE;

	float tick_offs = 0.0
		+ texture(t_noise, scaled_pos * 0.0005 - time_of_day.x * 0.00001).x * 0.5
		+ texture(t_noise, scaled_pos * 0.0015).x * 0.15;

	float value = (
		0.0
		+ texture(t_noise, scaled_pos * 0.0003 + tick_offs).x
		+ texture(t_noise, scaled_pos * 0.0015 - tick_offs * 2.0).x * 0.5
	) / 3.0;

	value += (0.0
		+ texture(t_noise, scaled_pos * 0.008 + time_of_day.x * 0.0002).x * 0.25
		+ texture(t_noise, scaled_pos * 0.02 + tick_offs + time_of_day.x * 0.0002).x * 0.15
	) * value;

	float density = max((value - CLOUD_THRESHOLD) - abs(pos.z - CLOUD_AVG_HEIGHT) / 200.0, 0.0) * CLOUD_DENSITY;

	const float SHADE_GRADIENT = 1.5 / (CLOUD_AVG_HEIGHT - CLOUD_HEIGHT_MIN);
	float shade = ((pos.z - CLOUD_AVG_HEIGHT) / (CLOUD_HEIGHT_MAX - CLOUD_HEIGHT_MIN)) * 2.5 + 0.7;

	return vec2(shade, density / (1.0 + vsum(abs(pos - cam_pos.xyz)) / 5000));
}

vec4 get_cloud_color(vec3 dir, vec3 origin, float time_of_day, float max_dist, float quality) {
	const int ITERS = 12;
	const float INCR = 1.0 / ITERS;

	float mind = (CLOUD_HEIGHT_MIN - origin.z) / dir.z;
	float maxd = (CLOUD_HEIGHT_MAX - origin.z) / dir.z;

	float start = max(min(mind, maxd), 0.0);
	float delta = min(abs(mind - maxd), max_dist);

	float fuzz = sin(texture(t_noise, dir.xz * 100000.0 + tick.x).x * 100.0) * INCR * delta * pow(abs(maxd - mind), 0.3) * 2.0;

	float cloud_shade = 1.0;
	float passthrough = 1.0;
	if ((mind > 0.0 || maxd > 0.0) && start < max_dist) {
		float dist = start;
		for (int i = 0; i < ITERS; i ++) {
			dist += fuzz * 0.01 * min(pow(dist * 0.005, 2.0), 1.0);

			vec3 pos = origin + dir * min(dist, max_dist);
			vec2 sample = cloud_at(pos);

			float integral = sample.y * INCR;
			passthrough *= 1.0 - integral;
			cloud_shade = mix(cloud_shade, sample.x, passthrough * integral);
			dist += INCR * delta;

			if (passthrough < 0.1) {
				break;
			}
		}
	}

	float total_density = 1.0 - passthrough / (1.0 + delta * 0.0001);

	total_density = max(total_density - 1.0 / pow(max_dist, 0.25), 0.0); // Hack

	return vec4(vec3(cloud_shade), total_density);
}
