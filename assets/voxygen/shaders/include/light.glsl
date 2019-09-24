struct Light {
	vec4 light_pos;
	vec4 light_col;
};

layout (std140)
uniform u_lights {
	Light lights[32];
};

#include <srgb.glsl>

vec3 illuminate(vec3 color, vec3 diffuse, vec3 ambience) {
	float avg_col = (color.r + color.g + color.b) / 3.0;
	return ((color - avg_col) * ambience * 5.0 + (diffuse + ambience) * avg_col) * (diffuse + ambience);
}

float attenuation_strength(vec3 rpos) {
	return 1.0 / (rpos.x * rpos.x + rpos.y * rpos.y + rpos.z * rpos.z);
}

vec3 light_at(vec3 wpos, vec3 wnorm) {
	const float LIGHT_AMBIENCE = 0.025;

	vec3 light = vec3(0);

	for (uint i = 0u; i < light_count.x; i++) {

		// Only access the array once
		Light L = lights[i];

		vec3 light_pos = L.light_pos.xyz;

		// Pre-calculate difference between light and fragment
		vec3 difference = light_pos - wpos;

		float strength = attenuation_strength(difference);

		// Multiply the vec3 only once
		vec3 color = srgb_to_linear(L.light_col.rgb) * (strength * L.light_col.a);

		// This is commented out to avoid conditional branching. See here: https://community.khronos.org/t/glsl-float-multiply-by-zero/104391
		// if (max(max(color.r, color.g), color.b) < 0.002) {
		// 	continue;
		// }

		// Old: light += color * clamp(dot(normalize(difference), wnorm), LIGHT_AMBIENCE, 1.0);

		// The dot product cannot be greater than one, so no need to clamp max value
		// Also, rather than checking if it is smaller than LIGHT_AMBIENCE, add LIGHT_AMBIENCE instead
		light += color * (max(0, dot(normalize(difference), wnorm)) + LIGHT_AMBIENCE);
	}
	return light;
}
