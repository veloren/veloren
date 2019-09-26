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
	uvec4 light_shadow_count;
	uvec4 medium;
	ivec4 select_pos;
};
