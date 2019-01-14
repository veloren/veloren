#version 330 core

in vec3 f_pos;
in vec3 f_norm;
in vec3 f_col;

layout (std140)
uniform u_locals {
	mat4 model_mat;
};

layout (std140)
uniform u_globals {
	mat4 view_mat;
	mat4 proj_mat;
	vec4 cam_pos;
	vec4 focus_pos;
	vec4 view_distance;
	vec4 time_of_day;
	vec4 tick;
};

out vec4 tgt_color;

void main() {
	vec3 world_norm = (model_mat * vec4(f_norm, 0.0)).xyz;

	float ambient = 0.5;

	vec3 sun_dir = normalize(vec3(1.3, 1.7, 1.1));

	float sun_diffuse = dot(sun_dir, world_norm) * 0.5;

	tgt_color = vec4(f_col * (ambient + sun_diffuse), 1.0);
}
