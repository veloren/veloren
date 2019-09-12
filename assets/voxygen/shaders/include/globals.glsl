layout (std140)
uniform u_globals {
	mat4 view_mat;
	mat4 proj_mat;
	vec4 cam_pos;
	vec4 focus_pos;
	vec4 view_distance;
	vec4 time_of_day;
	vec4 tick;
	vec4 screen_res;
	uvec4 light_count;
	uvec4 medium;
};

float vmax(vec3 v) {
	return max(max(v.x, v.y), v.z);
}

vec3 warpify(vec3 f_pos) {
	float dist = distance(cam_pos.xyz - f_pos);
	return f_pos + vec3(0, 0, -1) * pow(dist * 0.01, 2.0) * 20.0;
}
