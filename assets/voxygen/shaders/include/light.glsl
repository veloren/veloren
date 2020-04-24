#include <srgb.glsl>

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

float attenuation_strength(vec3 rpos) {
    // Idea: we start off with attenuation strength equal to 1 at the source of the point light.
	// return 1.0 / (1.0 + /*pow*/(rpos.x * rpos.x + rpos.y * rpos.y + rpos.z * rpos.z/*, 0.6*/));
	return 1.0 / (/*pow*/(rpos.x * rpos.x + rpos.y * rpos.y + rpos.z * rpos.z/*, 0.6*/));
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

		float strength = pow(attenuation_strength(difference), 0.6);

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

// Returns computed maximum intensity.
float lights_at(vec3 wpos, vec3 wnorm, vec3 cam_to_frag, vec3 k_a, vec3 k_d, vec3 k_s, float alpha, inout vec3 emitted_light, inout vec3 reflected_light/*, out float shadow*/) {
	// shadow = 0.0;
    vec3 ambient_light = vec3(0.0);

	const float LIGHT_AMBIENCE = 0.025;

	for (uint i = 0u; i < light_shadow_count.x; i ++) {

		// Only access the array once
		Light L = lights[i];

		vec3 light_pos = L.light_pos.xyz;

		// Pre-calculate difference between light and fragment
		vec3 difference = light_pos - wpos;
        float distance_2 = dot(difference, difference);

		// float strength = attenuation_strength(difference);// pow(attenuation_strength(difference), 0.6);
        // // NOTE: This normalizes strength to 1.0 at the center of the point source.
        // float strength = 1.0 / (1.0 + distance_2);
        float strength = 1.0 / distance_2;

		// Multiply the vec3 only once
		vec3 color = /*srgb_to_linear*/(L.light_col.rgb) * (13.0 * strength * L.light_col.a * L.light_col.a/* * L.light_col.a*/);

		// // Only access the array once
		// Shadow S = shadows[i];

		// vec3 shadow_pos = S.shadow_pos_radius.xyz;
		// float radius = S.shadow_pos_radius.w;

		// vec3 diff = shadow_pos - wpos;
		// if (diff.z >= 0.0) {
		// 	diff.z = -sign(diff.z) * diff.z * 0.1;
		// }

		// float shade = max(pow(diff.x * diff.x + diff.y * diff.y + diff.z * diff.z, 0.25) / pow(radius * radius * 0.5, 0.25), /*0.5*/0.0);

		// shadow = min(shadow, shade);

        // Compute reflectance.
        vec3 light_dir = -difference / sqrt(distance_2); // normalize(-difference);
        // light_dir = faceforward(light_dir, wnorm, light_dir);
        reflected_light += color * (strength == 0.0 ? vec3(1.0) : light_reflection_factor(wnorm, cam_to_frag, light_dir, k_d, k_s, alpha));

		// light += color * (max(0, max(dot(normalize(difference), wnorm), 0.15)) + LIGHT_AMBIENCE);
        // Compute emiittance.
        // float ambient_sides = clamp(mix(0.15, 0.0, abs(dot(wnorm, light_dir)) * 10000.0), 0.0, 0.15);
        float ambient_sides = 0.0;// max(dot(wnorm, light_dir) - 0.15, 0.15);
        // float ambient_sides = 0.0;
        ambient_light += color * (ambient_sides + LIGHT_AMBIENCE);
	}

    // shadow = shadow_at(wpos, wnorm);
    // float shadow = shadow_at(wpos, wnorm);
    // emitted_light += k_a * ambient_light/* * shadow*/;// min(shadow, 1.0);
    return 1.0;//ambient_light;
}
