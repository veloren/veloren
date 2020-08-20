layout (std140)
uniform u_globals {
	mat4 view_mat;
	mat4 proj_mat;
	mat4 all_mat;
	vec4 cam_pos;
	vec4 focus_off;
	vec4 focus_pos;
	vec4 view_distance;
	vec4 time_of_day;
	vec4 sun_dir;
	vec4 moon_dir;
	vec4 tick;
	vec4 screen_res;
	uvec4 light_shadow_count;
    vec4 shadow_proj_factors;
	uvec4 medium;
	ivec4 select_pos;
	vec4 gamma;
	// 0 - FirstPerson
	// 1 - ThirdPerson
	uint cam_mode;
	float sprite_render_distance;
};

// Specifies the pattern used in the player dithering
mat4 threshold_matrix = mat4(
	vec4(1.0 / 17.0,  9.0 / 17.0,  3.0 / 17.0, 11.0 / 17.0),
	vec4(13.0 / 17.0,  5.0 / 17.0, 15.0 / 17.0,  7.0 / 17.0),
	vec4(4.0 / 17.0, 12.0 / 17.0,  2.0 / 17.0, 10.0 / 17.0),
	vec4(16.0 / 17.0,  8.0 / 17.0, 14.0 / 17.0,  6.0 / 17.0)
);
float distance_divider = 2;
float shadow_dithering = 0.5;
