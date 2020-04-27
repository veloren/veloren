#version 330 core

#include <globals.glsl>
#include <random.glsl>

in vec3 f_pos;
in vec3 f_chunk_pos;
flat in uint f_pos_norm;
in float f_alt;
in vec4 f_shadow;
in vec3 f_col;
in float f_light;
in float f_ao;

out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

void main() {
	// First 3 normals are negative, next 3 are positive
	vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));

	// TODO: last 3 bits in v_pos_norm should be a number between 0 and 5, rather than 0-2 and a direction.
	uint norm_axis = (f_pos_norm >> 30) & 0x3u;
	// Increase array access by 3 to access positive values
	uint norm_dir = ((f_pos_norm >> 29) & 0x1u) * 3u;
	// Use an array to avoid conditional branching
	vec3 f_norm = normals[(f_pos_norm >> 29) & 0x7u];

    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    // vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    // vec3 view_dir = normalize(-vec3(vert_pos4)/* / vert_pos4.w*/);
    vec3 view_dir = -cam_to_frag;
    // vec3 view_dir = normalize(f_pos - cam_pos.xyz);

    vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x);
    // float sun_light = get_sun_brightness(sun_dir);
	// float moon_light = get_moon_brightness(moon_dir);
    /* float sun_shade_frac = horizon_at(f_pos, sun_dir);
    float moon_shade_frac = horizon_at(f_pos, moon_dir); */
    // float f_alt = alt_at(f_pos.xy);
    // vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
    float moon_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, moon_dir);
    // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-f_norm, sun_dir)) * 10000.0), 0.0, 0.5);
    // NOTE: current assumption is that moon and sun shouldn't be out at the sae time.
    // This assumption is (or can at least easily be) wrong, but if we pretend it's true we avoids having to explicitly pass in a separate shadow
    // for the sun and moon (since they have different brightnesses / colors so the shadows shouldn't attenuate equally).
    float shade_frac = /*1.0;*/sun_shade_frac + moon_shade_frac;

    vec3 surf_color = /*srgb_to_linear*/(f_col);
    float alpha = 1.0;
    // TODO: Possibly angle with water surface into account?  Since we can basically assume it's horizontal.
    const float n2 = 1.01;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float R_s = (f_pos.z < f_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);
    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);
    float max_light = 0.0;

    vec3 emitted_light, reflected_light;
    // To account for prior saturation
    float f_light = pow(f_light, 1.5);
    float point_shadow = shadow_at(f_pos, f_norm);
    max_light += get_sun_diffuse2(f_norm, /*time_of_day.x, */sun_dir, moon_dir, view_dir, k_a/* * (shade_frac * 0.5 + light_frac * 0.5)*/, k_d, k_s, alpha, emitted_light, reflected_light);

    emitted_light *= f_light * point_shadow * max(shade_frac, MIN_SHADOW);
    reflected_light *= f_light * point_shadow * shade_frac;
    max_light *= f_light * point_shadow * shade_frac;

    max_light += lights_at(f_pos, f_norm, view_dir, k_a, k_d, k_s, alpha, emitted_light, reflected_light);

	float ao = /*pow(f_ao, 0.5)*/f_ao * 0.9 + 0.1;
	emitted_light *= ao;
	reflected_light *= ao;
    /* vec3 point_light = light_at(f_pos, f_norm);
    emitted_light += point_light;
    reflected_light += point_light; */

	// float point_shadow = shadow_at(f_pos, f_norm);
	// vec3 point_light = light_at(f_pos, f_norm);
	// vec3 light, diffuse_light, ambient_light;

    // get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, k_a * f_light, k_d * f_light, k_s * f_light, alpha, emitted_light, reflected_light);
	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	// float point_shadow = shadow_at(f_pos, f_norm);
	// diffuse_light *= f_light * point_shadow;
	// ambient_light *= f_light * point_shadow;
	// vec3 point_light = light_at(f_pos, f_norm);
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;
    // reflected_light += light_reflection_factor(norm, cam_to_frag, , vec3 k_d, vec3 k_s, float alpha) {

    // light_reflection_factorplight_reflection_factor

	// vec3 surf_color = illuminate(srgb_to_linear(f_col), light, diffuse_light, ambient_light);
	vec3 col = srgb_to_linear(f_col + hash(vec4(floor(f_chunk_pos * 3.0 - f_norm * 0.5), 0)) * 0.02); // Small-scale noise
	surf_color = illuminate(max_light, col * emitted_light, col * reflected_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag/*view_dir*/, time_of_day.x, cam_pos.xyz, f_pos, 1.0, true, clouds);
    // vec3 color = surf_color;
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);

	tgt_color = vec4(color, 1.0);
}
