vec3 rand_perm_3(vec3 pos) {
	return sin(pos * vec3(1473.7 * pos.z + 472.3, 8891.1 * pos.x + 723.1, 3813.3 * pos.y + 982.5));
}

vec4 rand_perm_4(vec4 pos) {
	return sin(473.3 * pos * vec4(317.3 * pos.w + 917.7, 1473.7 * pos.z + 472.3, 8891.1 * pos.x + 723.1, 3813.3 * pos.y + 982.5) / pos.yxwz);
}

vec3 smooth_rand(vec3 pos, float lerp_axis) {
	vec3 r0 = rand_perm_3(vec3(pos.x, pos.y, pos.z) + floor(lerp_axis));
	vec3 r1 = rand_perm_3(vec3(pos.x, pos.y, pos.z) + floor(lerp_axis + 1.0));
	return r0 + (r1 - r0) * fract(lerp_axis);
}
