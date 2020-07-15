#include <srgb.glsl>
#include <shadows.glsl>

struct Light {
	vec4 light_pos;
	vec4 light_col;
    // mat4 light_proj;
};

layout (std140)
uniform u_lights {
	Light lights[31];
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

// // Compute attenuation due to light passing through a substance that fills an area below a horizontal plane
// // (e.g. in most cases, water below the water surface depth).
// //
// // wpos is the position of the point being hit.
// // ray_dir is the reversed direction of the ray (going "out" of the point being hit).
// // surface_alt is the estimated altitude of the horizontal surface separating the substance from air.
// // defaultpos is the position to use in computing the distance along material at this point if there was a failure.
// //
// // Ideally, defaultpos is set so we can avoid branching on error.
// float compute_attenuation_beam(vec3 wpos, vec3 ray_dir, float surface_alt, vec3 defaultpos, float attenuation_depth) {
//     vec3 water_intersection_surface_camera = vec3(cam_pos);
//     bool _water_intersects_surface_camera = IntersectRayPlane(f_pos, view_dir, vec3(0.0, 0.0, /*f_alt*/f_pos.z + f_light), cam_surface_dir, water_intersection_surface_camera);
//     // Should work because we set it up so that if IntersectRayPlane returns false for camera, its default intersection point is cam_pos.
//     float water_depth_to_camera = length(water_intersection_surface_camera - f_pos);
//
//     vec3 water_intersection_surface_light = f_pos;
//     bool _light_intersects_surface_water = IntersectRayPlane(f_pos, sun_dir.z <= 0.0 ? sun_dir : moon_dir, vec3(0.0, 0.0, /*f_alt*/f_pos.z + f_light), vec3(0.0, 0.0, 1.0), water_intersection_surface_light);
//     // Should work because we set it up so that if IntersectRayPlane returns false for light, its default intersection point is f_pos--
//     // i.e. if a light ray can't hit the water, it shouldn't contribute to coloring at all.
//     float water_depth_to_light = length(water_intersection_surface_light - f_pos);
//
//     // For ambient color, we just take the distance to the surface out of laziness.
//     float water_depth_to_vertical = max(/*f_alt - f_pos.z*/f_light, 0.0);
//
//     // Color goes down with distance...
//     // See https://en.wikipedia.org/wiki/Beer%E2%80%93Lambert_law.
//     vec3 water_color_direct = exp(-water_attenuation * (water_depth_to_light + water_depth_to_camera));
//     vec3 water_color_ambient = exp(-water_attenuation * (water_depth_to_vertical + water_depth_to_camera));
//
// }

vec3 light_at(vec3 wpos, vec3 wnorm) {
	const float LIGHT_AMBIANCE = 0.025;

	vec3 light = vec3(0);

	for (uint i = 0u; i < light_shadow_count.x; i ++) {

		// Only access the array once
		Light L = lights[i];

		vec3 light_pos = L.light_pos.xyz - focus_off.xyz;

		// Pre-calculate difference between light and fragment
		vec3 difference = light_pos - wpos;

		float strength = attenuation_strength(difference);

		// Multiply the vec3 only once
		vec3 color = srgb_to_linear(L.light_col.rgb) * (strength * L.light_col.a);

		light += color * (max(0, max(dot(normalize(difference), wnorm), 0.15)) + LIGHT_AMBIANCE);
	}
	return light;
}

float shadow_at(vec3 wpos, vec3 wnorm) {
	float shadow = 1.0;

#if (SHADOW_MODE == SHADOW_MODE_NONE || SHADOW_MODE == SHADOW_MODE_MAP)
    return shadow;
#elif (SHADOW_MODE == SHADOW_MODE_CHEAP)
	for (uint i = 0u; i < light_shadow_count.y; i ++) {

		// Only access the array once
		Shadow S = shadows[i];

		vec3 shadow_pos = S.shadow_pos_radius.xyz - focus_off.xyz;
		float radius = S.shadow_pos_radius.w;

		vec3 diff = shadow_pos - wpos;
		if (diff.z >= 0.0) {
			diff.z = -sign(diff.z) * diff.z * 0.1;
		}

		float shade = max(pow(diff.x * diff.x + diff.y * diff.y + diff.z * diff.z, 0.25) / pow(radius * radius * 0.5, 0.25), 0.5);
		// float shade = max(pow(dot(diff, diff) / (radius * radius * 0.5), 0.25), 0.5);
        // float shade = dot(diff, diff) / (radius * radius * 0.5);

		shadow = min(shadow, shade);
	}
    // NOTE: Squared to compenate for prior saturation.
	return min(shadow, 1.0);
	// return min(shadow * shadow, 1.0);
#endif
}

// Returns computed maximum intensity.
//
// mu is the attenuation coefficient for any substance on a horizontal plane.
// cam_attenuation is the total light attenuation due to the substance for beams between the point and the camera.
// surface_alt is the altitude of the attenuating surface.
float lights_at(vec3 wpos, vec3 wnorm, vec3 /*cam_to_frag*/view_dir, vec3 mu, vec3 cam_attenuation, float surface_alt, vec3 k_a, vec3 k_d, vec3 k_s, float alpha, vec3 voxel_norm, float voxel_lighting, inout vec3 emitted_light, inout vec3 reflected_light/*, out float shadow*/) {
    // return 0.0;
	// shadow = 0.0;
    // vec3 ambient_light = vec3(0.0);
    vec3 directed_light = vec3(0.0);
    vec3 max_light = vec3(0.0);

	const float LIGHT_AMBIANCE = 0.0;//0.015625;

	for (uint i = 0u; i < /*light_shadow_count.x*//*0u*/light_shadow_count.x/*32u*/; i ++) {

		// Only access the array once
		Light L = lights[i];

		vec3 light_pos = L.light_pos.xyz - focus_off.xyz;

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
        float square_factor = /*2.0 * PI_2 * *//*2.0 * */L.light_col.a;
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
        float light_distance = sqrt(distance_2);
        vec3 light_dir = -difference / light_distance; // normalize(-difference);
        // light_dir = faceforward(light_dir, wnorm, light_dir);
        bool is_direct = dot(-light_dir, wnorm) > 0.0;
        // reflected_light += color * (distance_2 == 0.0 ? vec3(1.0) : light_reflection_factor(wnorm, cam_to_frag, light_dir, k_d, k_s, alpha));
        vec3 direct_light_dir = is_direct ? light_dir : -light_dir;
        // vec3 direct_norm_dir = is_direct ? wnorm : -wnorm;
        // Compute attenuation due to fluid.
        // Default is light_pos, so we take the whole segment length for this beam if it never intersects the surface, unlesss the beam itself
        // is above the surface, in which case we take zero (wpos).
        color *= cam_attenuation * compute_attenuation_point(wpos, -direct_light_dir, mu, surface_alt, light_pos.z < surface_alt ? light_pos : wpos);

#if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
        is_direct = true;
#endif
        vec3 direct_light = PI * color * strength * square_factor * light_reflection_factor(/*direct_norm_dir*/wnorm, /*cam_to_frag*/view_dir, direct_light_dir, k_d, k_s, alpha, voxel_norm, voxel_lighting);
        float computed_shadow = ShadowCalculationPoint(i, -difference, wnorm, wpos/*, light_distance*/);
        // directed_light += is_direct ? max(computed_shadow, /*LIGHT_AMBIANCE*/0.0) * direct_light * square_factor : vec3(0.0);
        directed_light += is_direct ? mix(LIGHT_AMBIANCE, 1.0, computed_shadow) * direct_light * square_factor : vec3(0.0);
        // directed_light += (is_direct ? 1.0 : LIGHT_AMBIANCE) * max(computed_shadow, /*LIGHT_AMBIANCE*/0.0) * direct_light * square_factor;// : vec3(0.0);
        // directed_light += mix(LIGHT_AMBIANCE, 1.0, computed_shadow) * direct_light * square_factor;
        // ambient_light += is_direct ? vec3(0.0) : vec3(0.0); // direct_light * square_factor * LIGHT_AMBIANCE;
        // ambient_light += is_direct ? direct_light * (1.0 - square_factor * LIGHT_AMBIANCE) : vec3(0.0);

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
        max_light += /*max(1.0, cam_strength)*//*min(cam_strength, 1.0)*//*max*//*max(both_strength, 1.0) * *//*cam_strength*/computed_shadow * both_strength * square_factor * square_factor * PI * color;
        // max_light += /*max(1.0, cam_strength)*//*min(cam_strength, 1.0)*//*max*/max(cam_strength, 1.0/*, strength*//*1.0*/) * square_factor * square_factor * PI * color;
		// light += color * (max(0, max(dot(normalize(difference), wnorm), 0.15)) + LIGHT_AMBIANCE);
        // Compute emiittance.
        // float ambient_sides = clamp(mix(0.15, 0.0, abs(dot(wnorm, light_dir)) * 10000.0), 0.0, 0.15);
        // float ambient_sides = 0.0;// max(dot(wnorm, light_dir) - 0.15, 0.15);
        // // float ambient_sides = 0.0;
        // ambient_light += color * (ambient_sides + LIGHT_AMBIANCE);
	}

    // shadow = shadow_at(wpos, wnorm);
    // float shadow = shadow_at(wpos, wnorm);
    reflected_light += directed_light;
    // emitted_light += k_a * ambient_light/* * shadow*/;// min(shadow, 1.0);
    return /*rel_luminance(ambient_light + directed_light)*/rel_luminance(max_light);//ambient_light;
}

// Same as lights_at, but with no assumed attenuation due to fluid.
float lights_at(vec3 wpos, vec3 wnorm, vec3 view_dir, vec3 k_a, vec3 k_d, vec3 k_s, float alpha, inout vec3 emitted_light, inout vec3 reflected_light) {
    return lights_at(wpos, wnorm, view_dir, vec3(0.0), vec3(1.0), 0.0, k_a, k_d, k_s, alpha, wnorm, 1.0, emitted_light, reflected_light);
}
