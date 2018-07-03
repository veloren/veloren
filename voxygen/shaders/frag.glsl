#version 330 core

in vec3 frag_pos;
in vec3 frag_norm;
in vec4 frag_col;

uniform constants {
	mat4 model_mat;
    mat4 view_mat;
	mat4 perspective_mat;
};

out vec4 target;

float diffuse_factor = 0.5;
float ambient_factor = 0.4;
vec3  sun_direction  = normalize(vec3(1, -0.7, -1.4));
vec3  sun_color      = vec3(1, 1, 1);
float sun_specular   = 0.3;
float sun_factor     = 10;
float sun_shine      = 0;

void main() {
    target = frag_col;

	// Geometry
	vec3 world_norm = normalize((model_mat * vec4(frag_norm, 0)).xyz);
	vec3 cam_pos = (view_mat * model_mat * vec4(frag_pos, 1)).xyz;

	// Ambiant light
	vec3 ambient = frag_col.xyz * ambient_factor * sun_color;

	// Diffuse light
	vec3 diffuse = frag_col.xyz * diffuse_factor * sun_color * max(0, dot(world_norm, -normalize(sun_direction)));

	// Specular light
	vec3 reflect_vec = (view_mat * vec4(reflect(sun_direction, world_norm), 0)).xyz;
	float specular_val = clamp(dot(-normalize(cam_pos), reflect_vec) + sun_shine, 0, 1);
	vec3 specular = sun_color * pow(specular_val, sun_factor) * sun_specular;

	// Final fragment color
	target = vec4(ambient + diffuse + specular, frag_col.w);
}
