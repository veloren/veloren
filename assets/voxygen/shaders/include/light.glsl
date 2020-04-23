struct Light {
	vec4 light_pos;
	vec4 light_col;
};

layout (std140)
uniform u_lights {
	Light lights[32];
};

struct Shadow {
	vec4 shadow_pos_radius;
};

layout (std140)
uniform u_shadows {
	Shadow shadows[24];
};

#include <srgb.glsl>

vec3 illuminate(vec3 color, vec3 light, vec3 diffuse, vec3 ambience) {
	float avg_col = (color.r + color.g + color.b) / 3.0;
	return ((color - avg_col) * light + (diffuse + ambience) * avg_col) * (diffuse + ambience);
}

float attenuation_strength(vec3 rpos) {
	// This is not how light attenuation works at all, but it produces visually pleasing and mechanically useful properties
	float d2 = rpos.x * rpos.x + rpos.y * rpos.y + rpos.z * rpos.z;
	return max(2.0 / pow(d2 + 10, 0.35) - pow(d2 / 50000.0, 0.8), 0.0);
}

vec3 light_at(vec3 wpos, vec3 wnorm) {
	const float LIGHT_AMBIENCE = 0.025;

	vec3 light = vec3(0);

	for (uint i = 0u; i < light_shadow_count.x; i ++) {

		// Only access the array once
		Light L = lights[i];

		vec3 light_pos = L.light_pos.xyz;

		// Pre-calculate difference between light and fragment
		vec3 difference = light_pos - wpos;

		float strength = attenuation_strength(difference);

		// Multiply the vec3 only once
		vec3 color = srgb_to_linear(L.light_col.rgb) * (strength * L.light_col.a);

		light += color * (max(0, max(dot(normalize(difference), wnorm), 0.15)) + LIGHT_AMBIENCE);
	}
	return light;
}

float shadow_at(vec3 wpos, vec3 wnorm) {
	float shadow = 1.0;

	for (uint i = 0u; i < light_shadow_count.y; i ++) {

		// Only access the array once
		Shadow S = shadows[i];

		vec3 shadow_pos = S.shadow_pos_radius.xyz;
		float radius = S.shadow_pos_radius.w;

		vec3 diff = shadow_pos - wpos;
		if (diff.z >= 0.0) {
			diff.z = -sign(diff.z) * diff.z * 0.1;
		}

		float shade = max(pow(diff.x * diff.x + diff.y * diff.y + diff.z * diff.z, 0.25) / pow(radius * radius * 0.5, 0.25), 0.5);

		shadow = min(shadow, shade);
	}
	return min(shadow, 1.0);
}
