const float PI = 3.141592;

vec3 get_sky_color(vec3 dir, float time_of_day) {
	const float TIME_FACTOR = (PI * 2.0) / (3600.0 * 24.0);

	const vec3 SKY_TOP    = vec3(0.1, 0.5, 1.0);
	const vec3 SKY_BOTTOM = vec3(0.025, 0.08, 0.2);

	const vec3 SUN_HALO_COLOR = vec3(1.0, 0.7, 0.5);
	const vec3 SUN_SURF_COLOR = vec3(1.0, 0.9, 0.35) * 200.0;

	float sun_angle_rad = time_of_day * TIME_FACTOR;
	vec3 sun_dir = vec3(sin(sun_angle_rad), 0.0, cos(sun_angle_rad));

	vec3 sun_halo = pow(max(dot(dir, sun_dir), 0.0), 8.0) * SUN_HALO_COLOR;
	vec3 sun_surf = pow(max(dot(dir, sun_dir) - 0.0045, 0.0), 1000.0) * SUN_SURF_COLOR;
	vec3 sun_light = sun_halo + sun_surf;

	return mix(SKY_BOTTOM, SKY_TOP, (dir.z + 1.0) / 2.0) + sun_light;
}

float fog(vec2 f_pos, vec2 focus_pos) {
	float dist = distance(f_pos, focus_pos) / view_distance.x;
	float min_fog = 0.5;
	float max_fog = 1.0;

	return clamp((dist - min_fog) / (max_fog - min_fog), 0.0, 1.0);
}
