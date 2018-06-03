#version 330 core

in vec4 frag_col;
in vec3 frag_norm;

uniform constants {
    mat4 camera_mat;
	mat4 model_mat;
};

out vec4 target;

float diffuse_factor = 0.5;
float ambient_factor = 0.4;
vec3  sun_direction = normalize(vec3(1, -1, -1));
vec3  sun_color     = vec3(1, 1, 1);
float sun_factor    = 50;
float sun_shine = 0;

void main() {
    target = frag_col;

	// Geometry
	vec3 world_norm = normalize((model_mat * vec4(frag_norm, 0)).xyz);

	// Ambiant light
	vec3 ambient_light = frag_col.xyz * ambient_factor * sun_color;

	// Diffuse light
	vec3 diffuse_light = frag_col.xyz * diffuse_factor * sun_color * max(0, dot(world_norm, -normalize(sun_direction)));

	// Final fragment color
	target = vec4(ambient_light + diffuse_light, frag_col.w);
}
