uniform sampler2D t_noise;

const float CLOUD_AVG_HEIGHT = 1025.0;
const float CLOUD_HEIGHT_MIN = CLOUD_AVG_HEIGHT - 50.0;
const float CLOUD_HEIGHT_MAX = CLOUD_AVG_HEIGHT + 50.0;
const float CLOUD_THRESHOLD = 0.3;
const float CLOUD_SCALE = 5.0;
const float CLOUD_DENSITY = 50.0;

float vsum(vec3 v) {
	return v.x + v.y + v.z;
}

vec2 cloud_at(vec3 pos) {
	float tick_offs = 0.0
		+ texture(t_noise, pos.xy * 0.0001 - tick.x * 0.001).x * 0.5 
		+ texture(t_noise, pos.xy * 0.000003).x * 5.0;

	float value = (
		0.0
		+ texture(t_noise, pos.xy / CLOUD_SCALE * 0.0003 + tick_offs).x
		+ texture(t_noise, pos.xy / CLOUD_SCALE * 0.0009 - tick_offs).x * 0.5
		+ texture(t_noise, pos.xy / CLOUD_SCALE * 0.0025 - tick.x * 0.01).x * 0.25
        + texture(t_noise, pos.xy / CLOUD_SCALE * 0.008 + tick.x * 0.02).x * 0.15
        + texture(t_noise, pos.xy / CLOUD_SCALE * 0.02 + tick_offs + tick.x * 0.02).x * 0.1
	) / 3.0;

	float density = max((value - CLOUD_THRESHOLD) - abs(pos.z - CLOUD_AVG_HEIGHT) / 400.0, 0.0) * CLOUD_DENSITY;

	float shade = ((pos.z - CLOUD_AVG_HEIGHT) * 1.8 / (CLOUD_AVG_HEIGHT - CLOUD_HEIGHT_MIN) + 0.5);

	return vec2(shade, density / (1.0 + vsum(abs(pos - cam_pos.xyz)) / 5000));
}

vec4 get_cloud_color(vec3 dir, vec3 origin, float time_of_day, float max_dist, float quality) {

	const float INCR = 0.1;

	float mind = (CLOUD_HEIGHT_MIN - origin.z) / dir.z;
	float maxd = (CLOUD_HEIGHT_MAX - origin.z) / dir.z;

	float start = max(min(mind, maxd), 0.0);
	float delta = min(abs(mind - maxd), max_dist);

	bool do_cast = true;
	if (mind < 0.0 && maxd < 0.0) {
		do_cast = false;
	}

	float incr = INCR;

	float fuzz = sin(texture(t_noise, dir.xz * 100000.0 + tick.x).x * 100.0) * incr * delta;

	float cloud_shade = 1.0;
	float passthrough = 1.0;
	if (do_cast) {
		for (float d = 0.0; d < 1.0; d += incr) {
			float dist = start + d * delta;
            dist += fuzz * pow(maxd - mind, 0.5) * 0.01 * min(pow(dist * 0.005, 2.0), 1.0);

			vec3 pos = origin + dir * min(dist, max_dist);
			vec2 sample = cloud_at(pos);

			float integral = sample.y * incr;
			passthrough *= max(1.0 - integral, 0.0);
			cloud_shade = mix(cloud_shade, sample.x, passthrough * integral);
		}
	}

	float total_density = 1.0 - passthrough / (1.0 + delta * 0.0001);

	total_density = max(total_density - 1.0 / pow(max_dist, 0.25), 0.0); // Hack

	return vec4(vec3(cloud_shade), total_density);
}
