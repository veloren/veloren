#version 330 core

#include <globals.glsl>

in vec3 f_pos;
in vec3 f_col;
flat in vec3 f_norm;
in float f_alt;
in vec4 f_shadow;

layout (std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
};

struct BoneData {
	mat4 bone_mat;
};

layout (std140)
uniform u_bones {
	BoneData bones[16];
};

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

out vec4 tgt_color;

void main() {
    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    // vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    // vec3 view_dir = normalize(-vec3(vert_pos4)/* / vert_pos4.w*/);
    vec3 view_dir = -cam_to_frag;

    vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x);
    // float sun_light = get_sun_brightness(sun_dir);
	// float moon_light = get_moon_brightness(moon_dir);
    /* float sun_shade_frac = horizon_at(f_pos, sun_dir);
    float moon_shade_frac = horizon_at(f_pos, moon_dir); */
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
    float moon_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, moon_dir);
    // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-f_norm, sun_dir)) * 10000.0), 0.0, 0.5);
    // NOTE: current assumption is that moon and sun shouldn't be out at the sae time.
    // This assumption is (or can at least easily be) wrong, but if we pretend it's true we avoids having to explicitly pass in a separate shadow
    // for the sun and moon (since they have different brightnesses / colors so the shadows shouldn't attenuate equally).
    float shade_frac = /*1.0;*/sun_shade_frac + moon_shade_frac;

	vec3 surf_color = /*srgb_to_linear*/(model_col.rgb * f_col);
    float alpha = 1.0;
    const float n2 = 1.01;
    const float R_s = pow((1.0 - n2) / (1.0 + n2), 2);

    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);

    vec3 emitted_light, reflected_light;

	float point_shadow = shadow_at(f_pos, f_norm);
    vec3 light_frac = /*vec3(1.0);*//*vec3(max(dot(f_norm, -sun_dir) * 0.5 + 0.5, 0.0));*/light_reflection_factor(f_norm, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(R_s), alpha);
	// vec3 point_light = light_at(f_pos, f_norm);
	// vec3 light, diffuse_light, ambient_light;
    //get_sun_diffuse(f_norm, time_of_day.x, view_dir, k_a * point_shadow * (shade_frac * 0.5 + light_frac * 0.5), k_d * point_shadow * shade_frac, k_s * point_shadow * shade_frac, alpha, emitted_light, reflected_light);
    get_sun_diffuse2(f_norm, sun_dir, moon_dir, view_dir, k_a * (shade_frac * 0.5 + light_frac * 0.5), k_d, k_s, alpha, emitted_light, reflected_light);
    reflected_light *= point_shadow * shade_frac;
    emitted_light *= point_shadow;

    lights_at(f_pos, f_norm, view_dir, k_a, k_d, k_s, alpha, emitted_light, reflected_light);
    /* vec3 point_light = light_at(f_pos, f_norm);
    emitted_light += point_light;
    reflected_light += point_light; */
    // get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, surf_color * f_light * point_shadow, 0.5 * surf_color * f_light * point_shadow, 0.5 * surf_color * f_light * point_shadow, 2.0, emitted_light, reflected_light);

	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	// diffuse_light *= point_shadow;
	// ambient_light *= point_shadow;
	// vec3 point_light = light_at(f_pos, f_norm);
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;
	// vec3 surf_color = illuminate(srgb_to_linear(model_col.rgb * f_col), light, diffuse_light, ambient_light);
	surf_color = illuminate(surf_color * emitted_light, surf_color * reflected_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag/*view_dir*/, time_of_day.x, cam_pos.xyz, f_pos, 0.5, true, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);

	tgt_color = vec4(color, 1.0);
}
