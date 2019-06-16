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

vec3 get_sky_color(vec3 dir, float time_of_day) {
	bool objects = true;

	const float TIME_FACTOR = (PI * 2.0) / (3600.0 * 24.0);

	vec2 pos2d = dir.xy / dir.z;

	const vec3 SKY_TOP    = vec3(0.2, 0.3, 0.9);
	const vec3 SKY_MIDDLE = vec3(0.1, 0.15, 0.7);
	const vec3 SKY_BOTTOM = vec3(0.025, 0.15, 0.35);

	const vec3 SUN_HALO_COLOR = vec3(1.0, 0.7, 0.5) * 0.5;
	const vec3 SUN_SURF_COLOR = vec3(1.0, 0.9, 0.35) * 200.0;

	float sun_angle_rad = time_of_day * TIME_FACTOR;
	vec3 sun_dir = vec3(sin(sun_angle_rad), 0.0, cos(sun_angle_rad));

	vec3 sun_halo = pow(max(dot(dir, sun_dir), 0.0), 8.0) * SUN_HALO_COLOR;
	vec3 sun_surf = pow(max(dot(dir, sun_dir) - 0.0045, 0.0), 1000.0) * SUN_SURF_COLOR;
	vec3 sun_light = sun_halo + sun_surf;

	vec3 sky_top;
	if (objects) {
		vec3 p = vec3(pos2d + time_of_day * 0.0002, time_of_day * 0.0001);
		sky_top = mix(SKY_TOP, vec3(1), pow(noise(p) * 0.8 + noise(p * 3.0) * 0.2, 2.5) * 3.0);
	} else {
		sky_top = SKY_TOP;
	}

	vec3 sky_color = mix(mix(SKY_MIDDLE, sky_top, clamp(dir.z, 0, 1)), SKY_BOTTOM, clamp(-dir.z * 5.0, 0, 1));

	if (objects) {
		sky_color += sun_light;
	}

	return sky_color;
}

float fog(vec2 f_pos, vec2 focus_pos) {
	float dist = distance(f_pos, focus_pos) / view_distance.x;
	float min_fog = 0.75;
	float max_fog = 1.0;

	return clamp((dist - min_fog) / (max_fog - min_fog), 0.0, 1.0);
}
