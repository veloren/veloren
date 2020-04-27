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
    // NOTE: Squared to compenate for prior saturation.
	return min(shadow * shadow, 1.0);
}

// Returns computed maximum intensity.
float lights_at(vec3 wpos, vec3 wnorm, vec3 cam_to_frag, vec3 k_a, vec3 k_d, vec3 k_s, float alpha, inout vec3 emitted_light, inout vec3 reflected_light/*, out float shadow*/) {
	// shadow = 0.0;
    vec3 ambient_light = vec3(0.0);
    vec3 directed_light = vec3(0.0);
    vec3 max_light = vec3(0.0);

	const float LIGHT_AMBIENCE = 0.5;

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
        const float PI = 3.1415926535897932384626433832795;
        const float PI_2 = 2 * PI;
        float square_factor = /*2.0 * PI_2 * */2.0 * L.light_col.a;
		vec3 color = /*srgb_to_linear*/L.light_col.rgb;

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
        bool is_direct = dot(-light_dir, wnorm) > 0.0;
        // reflected_light += color * (distance_2 == 0.0 ? vec3(1.0) : light_reflection_factor(wnorm, cam_to_frag, light_dir, k_d, k_s, alpha));
        vec3 direct_light = PI * color * strength * square_factor * light_reflection_factor(wnorm, cam_to_frag, is_direct ? light_dir : -light_dir, k_d, k_s, alpha);
        directed_light += is_direct ? direct_light * square_factor : vec3(0.0);
        ambient_light += is_direct ? vec3(0.0) : direct_light * LIGHT_AMBIENCE;

        vec3 cam_light_diff = light_pos - focus_pos.xyz;
        float cam_distance_2 = dot(cam_light_diff, cam_light_diff);// + 0.0001;
        float cam_strength = 1.0 / (/*4.0 * *//*PI * *//*1.0 + */cam_distance_2);

        // vec3 cam_pos_diff  = cam_to_frag.xyz - wpos;
        // float pos_distance_2 = dot(cam_pos_diff, cam_pos_diff);// + 0.0001;

        // float cam_distance = sqrt(cam_distance_2);
        // float distance = sqrt(distance_2);
        float both_strength = cam_distance_2 == 0.0 ? distance_2 == 0.0 ? 0.0 : strength/* * strength*//*1.0*/ : distance_2 == 0.0 ? cam_strength/* * cam_strength*//*1.0*/ :
            // 1.0 / (cam_distance * distance);
            // sqrt(cam_strength * strength);
            cam_strength + strength;
            // (cam_strength * strength);
            // max(cam_strength, strength);
            // mix(cam_strength, strength, distance_2 / (cam_distance_2 + distance_2));
            // mix(cam_strength, strength, cam_distance_2 / (cam_distance_2 + distance_2));
            // max(cam_strength, strength);//mix(cam_strength, strength, clamp(distance_2 / /*pos_distance_2*/cam_distance_2, 0.0, 1.0));
        // float both_strength = mix(cam_strength, strength, cam_distance_2 / sqrt(cam_distance_2 + distance_2));
        max_light += /*max(1.0, cam_strength)*//*min(cam_strength, 1.0)*//*max*//*max(both_strength, 1.0) * *//*cam_strength*/both_strength * square_factor * square_factor * PI * color;
        // max_light += /*max(1.0, cam_strength)*//*min(cam_strength, 1.0)*//*max*/max(cam_strength, 1.0/*, strength*//*1.0*/) * square_factor * square_factor * PI * color;
		// light += color * (max(0, max(dot(normalize(difference), wnorm), 0.15)) + LIGHT_AMBIENCE);
        // Compute emiittance.
        // float ambient_sides = clamp(mix(0.15, 0.0, abs(dot(wnorm, light_dir)) * 10000.0), 0.0, 0.15);
        // float ambient_sides = 0.0;// max(dot(wnorm, light_dir) - 0.15, 0.15);
        // // float ambient_sides = 0.0;
        // ambient_light += color * (ambient_sides + LIGHT_AMBIENCE);
	}

    // shadow = shadow_at(wpos, wnorm);
    // float shadow = shadow_at(wpos, wnorm);
    reflected_light += directed_light;
    emitted_light += k_a * ambient_light/* * shadow*/;// min(shadow, 1.0);
    return /*rel_luminance(ambient_light + directed_light)*/rel_luminance(max_light);//ambient_light;
}
