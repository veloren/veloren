#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE (LIGHTING_TYPE_TRANSMISSION | LIGHTING_TYPE_REFLECTION)

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_SPECULAR

#if (FLUID_MODE == FLUID_MODE_CHEAP)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE == FLUID_MODE_SHINY)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#define HAS_SHADOW_MAPS

// https://www.shadertoy.com/view/XdsyWf

#include <globals.glsl>
#include <random.glsl>

in vec3 f_pos;
flat in uint f_pos_norm;
// in vec3 f_col;
// in float f_light;
// in vec3 light_pos[2];

//struct ShadowLocals {
//	mat4 shadowMatrices;
//    mat4 texture_mat;
//};
//
//layout (std140)
//uniform u_light_shadows {
//    ShadowLocals shadowMats[/*MAX_LAYER_FACES*/192];
//};

layout (std140)
uniform u_locals {
	vec3 model_offs;
	float load_time;
    ivec4 atlas_offs;
};

uniform sampler2D t_waves;

out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

vec3 warp_normal(vec3 norm, vec3 pos, float time) {
	return normalize(norm
		+ smooth_rand(pos * 1.0, time * 1.0) * 0.05
		+ smooth_rand(pos * 0.25, time * 0.25) * 0.1);
}

float wave_height(vec3 pos) {
	float timer = tick.x * 0.75;

	pos *= 0.5;
	vec3 big_warp = (
		texture(t_waves, fract(pos.xy * 0.03 + timer * 0.01)).xyz * 0.5 +
		texture(t_waves, fract(pos.yx * 0.03 - timer * 0.01)).xyz * 0.5 +
		vec3(0)
	);

	vec3 warp = (
		texture(t_noise, fract(pos.yx * 0.1 + timer * 0.02)).xyz * 0.3 +
		texture(t_noise, fract(pos.yx * 0.1 - timer * 0.02)).xyz * 0.3 +
		vec3(0)
	);

	float height = (
		(texture(t_noise, pos.xy * 0.03 + big_warp.xy + timer * 0.05).y - 0.5) * 1.0 +
		(texture(t_noise, pos.yx * 0.03 + big_warp.yx - timer * 0.05).y - 0.5) * 1.0 +
		(texture(t_waves, pos.xy * 0.1 + warp.xy + timer * 0.1).x - 0.5) * 0.5 +
		(texture(t_waves, pos.yx * 0.1 + warp.yx - timer * 0.1).x - 0.5) * 0.5 +
		(texture(t_noise, pos.yx * 0.3 + warp.xy * 0.5 + timer * 0.1).x - 0.5) * 0.2 +
		(texture(t_noise, pos.yx * 0.3 + warp.yx * 0.5 - timer * 0.1).x - 0.5) * 0.2 +
		(texture(t_noise, pos.yx * 1.0 + warp.yx * 0.0 - timer * 0.1).x - 0.5) * 0.05 +
		0.0
	);

	return pow(abs(height), 0.5) * sign(height) * 10.5;
}

void main() {
	// First 3 normals are negative, next 3 are positive
	vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));

	// TODO: last 3 bits in v_pos_norm should be a number between 0 and 5, rather than 0-2 and a direction.
	uint norm_axis = (f_pos_norm >> 30) & 0x3u;
	// Increase array access by 3 to access positive values
	uint norm_dir = ((f_pos_norm >> 29) & 0x1u) * 3u;
	// Use an array to avoid conditional branching
	vec3 f_norm = normals[norm_axis + norm_dir];

    // vec4 light_pos[2];
//#if (SHADOW_MODE == SHADOW_MODE_MAP)
//    // for (uint i = 0u; i < light_shadow_count.z; ++i) {
//    //     light_pos[i] = /*vec3(*/shadowMats[i].texture_mat * vec4(f_pos, 1.0)/*)*/;
//    // }
//    vec4 sun_pos = /*vec3(*/shadowMats[0].texture_mat * vec4(f_pos, 1.0)/*)*/;
//#elif (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_NONE)
//    vec4 sun_pos = vec4(0.0);
//#endif

	vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    // vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    // vec3 view_dir = normalize(-vec3(vert_pos4)/* / vert_pos4.w*/);
    vec3 view_dir = -cam_to_frag;
	float frag_dist = length(f_pos - cam_pos.xyz);

	vec3 b_norm;
	if (f_norm.z > 0.0) {
		b_norm = vec3(1, 0, 0);
	} else if (f_norm.x > 0.0) {
		b_norm = vec3(0, 1, 0);
	} else {
		b_norm = vec3(0, 0, 1);
	}
	vec3 c_norm = cross(f_norm, b_norm);

	float wave00 = wave_height(f_pos);
	float wave10 = wave_height(f_pos + vec3(0.1, 0, 0));
	float wave01 = wave_height(f_pos + vec3(0, 0.1, 0));

	float slope = abs(wave00 - wave10) * abs(wave00 - wave01);
	vec3 nmap = vec3(
		-(wave10 - wave00) / 0.1,
		-(wave01 - wave00) / 0.1,
		0.1 / slope
	);

	nmap = mix(f_norm, normalize(nmap), min(1.0 / pow(frag_dist, 0.75), 1));

	vec3 norm = vec3(0, 0, 1) * nmap.z + b_norm * nmap.x + c_norm * nmap.y;
    // vec3 norm = f_norm;

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE == FLUID_MODE_SHINY)
    float f_alt = alt_at(f_pos.xy);
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_CHEAP)
    float f_alt = f_pos.z;
#endif

    float fluid_alt = max(ceil(f_pos.z), floor(f_alt));// f_alt;//max(f_alt - f_pos.z, 0.0);
    const float alpha = 0.255/*/ / 4.0*//* / 4.0 / sqrt(2.0)*/;
    const float n2 = 1.3325;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float R_s = (f_pos.z < fluid_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    // Water is transparent so both normals are valid.
    vec3 cam_norm = faceforward(norm, norm, cam_to_frag);
    vec4 _clouds;
	vec3 reflect_ray_dir = reflect(cam_to_frag/*-view_dir*/, norm);
	vec3 refract_ray_dir = refract(cam_to_frag/*-view_dir*/, norm, 1.0 / n2);
    vec3 sun_view_dir = view_dir;///*sign(cam_pos.z - fluid_alt) * view_dir;*/cam_pos.z <= fluid_alt ? -view_dir : view_dir;
    // vec3 sun_view_dir = cam_pos.z <= fluid_alt ? -view_dir : view_dir;
    vec3 beam_view_dir = reflect_ray_dir;//cam_pos.z <= fluid_alt ? -refract_ray_dir : reflect_ray_dir;
    /* vec4 reflect_ray_dir4 = view_mat * vec4(reflect_ray_dir, 1.0);
    reflect_ray_dir = normalize(vec3(reflect_ray_dir4) / reflect_ray_dir4.w); */
	// vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    // Squared to account for prior saturation.
    float f_light = 1.0;// pow(f_light, 1.5);
	vec3 reflect_color = get_sky_color(/*reflect_ray_dir*/beam_view_dir, time_of_day.x, f_pos, vec3(-100000), 0.25, false, _clouds) * f_light;
    // /*const */vec3 water_color = srgb_to_linear(vec3(0.2, 0.5, 1.0));
    // /*const */vec3 water_color = srgb_to_linear(vec3(0.8, 0.9, 1.0));
    // NOTE: Linear RGB, attenuation coefficients for water at roughly R, G, B wavelengths.
    // See https://en.wikipedia.org/wiki/Electromagnetic_absorption_by_water
    // /*const */vec3 water_attenuation = MU_WATER;// vec3(0.8, 0.05, 0.01);
    // /*const */vec3 water_color = vec3(0.2, 0.95, 0.99);

    /* vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x); */
#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP)
    vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#elif (SHADOW_MODE == SHADOW_MODE_NONE)
    float sun_shade_frac = 1.0;//horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#endif
    float moon_shade_frac = 1.0;// horizon_at2(f_shadow, f_alt, f_pos, moon_dir);
    // float sun_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, sun_dir);
    // float moon_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, moon_dir);
    // float shade_frac = /*1.0;*/sun_shade_frac + moon_shade_frac;

    // DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, light_pos);
    float point_shadow = shadow_at(f_pos, f_norm);
    DirectionalLight sun_info = get_sun_info(sun_dir, point_shadow * sun_shade_frac, /*sun_pos*/f_pos);
    DirectionalLight moon_info = get_moon_info(moon_dir, point_shadow * moon_shade_frac/*, light_pos*/);

    // Hack to determine water depth: color goes down with distance through water, so
    // we assume water color absorption from this point a to some other point b is the distance
    // along the the ray from a to b where it intersects with the surface plane; if it doesn't,
    // then the whole segment from a to b is considered underwater.
    // TODO: Consider doing for point lights.
    // vec3 cam_surface_dir = faceforward(vec3(0.0, 0.0, 1.0), cam_to_frag, vec3(0.0, 0.0, 1.0));

    // vec3 water_intersection_surface_camera = vec3(cam_pos);
    // bool _water_intersects_surface_camera = IntersectRayPlane(f_pos, view_dir, vec3(0.0, 0.0, /*f_alt*/f_pos.z + f_light), cam_surface_dir, water_intersection_surface_camera);
    // // Should work because we set it up so that if IntersectRayPlane returns false for camera, its default intersection point is cam_pos.
    // float water_depth_to_camera = length(water_intersection_surface_camera - f_pos);

    // vec3 water_intersection_surface_light = f_pos;
    // bool _light_intersects_surface_water = IntersectRayPlane(f_pos, sun_dir.z <= 0.0 ? sun_dir : moon_dir, vec3(0.0, 0.0, /*f_alt*/f_pos.z + f_light), vec3(0.0, 0.0, 1.0), water_intersection_surface_light);
    // // Should work because we set it up so that if IntersectRayPlane returns false for light, its default intersection point is f_pos--
    // // i.e. if a light ray can't hit the water, it shouldn't contribute to coloring at all.
    // float water_depth_to_light = length(water_intersection_surface_light - f_pos);

    // // For ambient color, we just take the distance to the surface out of laziness.
    // float water_depth_to_vertical = max(/*f_alt - f_pos.z*/f_light, 0.0);

    // // Color goes down with distance...
    // // See https://en.wikipedia.org/wiki/Beer%E2%80%93Lambert_law.
    // vec3 water_color_direct = exp(-MU_WATER);//exp(-MU_WATER);//vec3(1.0);
    // vec3 water_color_direct = exp(-water_attenuation * (water_depth_to_light + water_depth_to_camera));
    // vec3 water_color_ambient = exp(-water_attenuation * (water_depth_to_vertical + water_depth_to_camera));
    vec3 mu = MU_WATER;
    // NOTE: Default intersection point is camera position, meaning if we fail to intersect we assume the whole camera is in water.
    vec3 cam_attenuation = compute_attenuation_point(f_pos, -view_dir, mu, fluid_alt, cam_pos.xyz);
    // float water_depth_to_vertical = max(/*f_alt - f_pos.z*/f_light, 0.0);
    // For ambient color, we just take the distance to the surface out of laziness.
    // See https://en.wikipedia.org/wiki/Beer%E2%80%93Lambert_law.
    // float water_depth_to_vertical = max(fluid_alt - cam_pos.z/*f_light*/, 0.0);
    // vec3 ambient_attenuation = exp(-mu * water_depth_to_vertical);

    // For ambient reflection, we just take the water

    vec3 k_a = vec3(1.0);
    // Oxygen is light blue.
    vec3 k_d = vec3(/*vec3(0.2, 0.9, 0.99)*/1.0);
    vec3 k_s = vec3(R_s);//2.0 * reflect_color;

	vec3 emitted_light, reflected_light;
	// vec3 light, diffuse_light, ambient_light;
    // vec3 light_frac = /*vec3(1.0);*/light_reflection_factor(f_norm/*vec3(0, 0, 1.0)*/, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(R_s), alpha);
    // 0 = 100% reflection, 1 = translucent water
    float passthrough = /*pow(*/dot(faceforward(norm, norm, cam_to_frag/*view_dir*/), -cam_to_frag/*view_dir*/)/*, 0.5)*/;

    float max_light = 0.0;
    max_light += get_sun_diffuse2(sun_info, moon_info, norm, /*time_of_day.x*/sun_view_dir, f_pos, mu, cam_attenuation, fluid_alt, k_a/* * (shade_frac * 0.5 + light_frac * 0.5)*/, vec3(k_d), /*vec3(f_light * point_shadow)*//*reflect_color*/k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);
    // reflected_light *= /*water_color_direct * */reflect_color * f_light * point_shadow * shade_frac;
    // emitted_light *= /*water_color_direct*//*ambient_attenuation * */f_light * point_shadow * max(shade_frac, MIN_SHADOW);
    // max_light *= f_light * point_shadow * shade_frac;
    // reflected_light *= /*water_color_direct * */reflect_color * f_light * point_shadow;
    // emitted_light *= /*water_color_direct*//*ambient_attenuation * */f_light * point_shadow;
    // max_light *= f_light * point_shadow;

    // vec3 diffuse_light_point = vec3(0.0);
    // max_light += lights_at(f_pos, cam_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, vec3(1.0), /*vec3(0.0)*/k_s, alpha, emitted_light, diffuse_light_point);

    // vec3 dump_light = vec3(0.0);
    // vec3 specular_light_point = vec3(0.0);
    // lights_at(f_pos, cam_norm, view_dir, mu, cam_attenuation, fluid_alt, vec3(0.0), vec3(0.0), /*vec3(1.0)*/k_s, alpha, dump_light, specular_light_point);
    // diffuse_light_point -= specular_light_point;
    // max_light += lights_at(f_pos, cam_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, /*k_d*/vec3(0.0), /*vec3(0.0)*/k_s, alpha, emitted_light, /*diffuse_light*/reflected_light);

    max_light += lights_at(f_pos, cam_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, /*k_d*//*vec3(0.0)*/k_d, /*vec3(0.0)*/k_s, alpha, f_norm, 1.0, emitted_light, /*diffuse_light*/reflected_light);

    float reflected_light_point = length(reflected_light);///*length*/(diffuse_light_point.r) + f_light * point_shadow;
    // TODO: See if we can be smarter about this using point light distances.
    // reflected_light += k_d * (diffuse_light_point/* + f_light * point_shadow * shade_frac*/) + /*water_color_ambient*/specular_light_point;

	/* vec3 point_light = light_at(f_pos, norm);
    emitted_light += point_light;
    reflected_light += point_light; */

	// get_sun_diffuse(norm, time_of_day.x, light, diffuse_light, ambient_light, 0.0);
	// diffuse_light *= f_light * point_shadow;
	// ambient_light *= f_light * point_shadow;
	// vec3 point_light = light_at(f_pos, norm);
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;
	// vec3 surf_color = srgb_to_linear(vec3(0.2, 0.5, 1.0)) * light * diffuse_light * ambient_light;
    vec3 surf_color = illuminate(max_light, view_dir, emitted_light/* * log(1.0 - MU_WATER)*/, /*cam_attenuation * *//*water_color * */reflect_color * reflected_light/* * log(1.0 - MU_WATER)*/);

    // passthrough = pow(passthrough, 1.0 / (1.0 + water_depth_to_camera));
    /* surf_color = cam_attenuation.g < 0.5 ?
        vec3(1.0, 0.0, 0.0) :
        vec3(0.0, 1.0, 1.0)
    ; */
    // passthrough = passthrough * length(cam_attenuation);

	// vec3 reflect_ray_dir = reflect(cam_to_frag, norm);
	// Hack to prevent the reflection ray dipping below the horizon and creating weird blue spots in the water
	// reflect_ray_dir.z = max(reflect_ray_dir.z, 0.01);

	// vec4 _clouds;
	// vec3 reflect_color = get_sky_color(reflect_ray_dir, time_of_day.x, f_pos, vec3(-100000), 0.25, false, _clouds) * f_light;
	// Tint
	// reflect_color = mix(reflect_color, surf_color, 0.6);

	// vec4 color = mix(vec4(reflect_color * 2.0, 1.0), vec4(surf_color, 1.0 / (1.0 + /*diffuse_light*/(f_light * point_shadow + point_light) * 0.25)), passthrough);
	// vec4 color = mix(vec4(reflect_color * 2.0, 1.0), vec4(surf_color, 1.0 / (1.0 + /*diffuse_light*/(/*f_light * point_shadow*/f_light * point_shadow + reflected_light_point/* + point_light*//*reflected_light*/) * 0.25)), passthrough);
    // vec4 color = mix(vec4(surf_color, 1.0), vec4(surf_color, 0.0), passthrough);
    //vec4 color = vec4(surf_color, 1.0);
	// vec4 color = mix(vec4(reflect_color, 1.0), vec4(surf_color, 1.0 / (1.0 + /*diffuse_light*/(/*f_light * point_shadow*/reflected_light_point/* + point_light*//*reflected_light*/))), passthrough);

    // float log_cam = log(min(cam_attenuation.r, min(cam_attenuation.g, cam_attenuation.b)));
    float min_refl = min(emitted_light.r, min(emitted_light.g, emitted_light.b));
    vec4 color = vec4(surf_color, passthrough * 1.0 / (1.0 + min_refl));// * (1.0 - /*log(1.0 + cam_attenuation)*//*cam_attenuation*/1.0 / (2.0 - log_cam)));
    // vec4 color = vec4(surf_color, mix(1.0, 1.0 / (1.0 + /*0.25 * *//*diffuse_light*/(/*f_light * point_shadow*/reflected_light_point)), passthrough));
    // vec4 color = vec4(surf_color, mix(1.0, length(cam_attenuation), passthrough));

	/* reflect_color = reflect_color * 0.5 * (diffuse_light + ambient_light);
	// 0 = 100% reflection, 1 = translucent water
	float passthrough = dot(faceforward(f_norm, f_norm, cam_to_frag), -cam_to_frag);

	vec4 color = mix(vec4(reflect_color, 1.0), vec4(vec3(0), 1.0 / (1.0 + diffuse_light * 0.25)), passthrough); */

#if (CLOUD_MODE == CLOUD_MODE_REGULAR)
    float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
    vec3 fog_color = get_sky_color(cam_to_frag/*-view_dir*/, time_of_day.x, cam_pos.xyz, f_pos, 0.25, false, clouds);
    vec4 final_color = mix(mix(color, vec4(fog_color, 0.0), fog_level), vec4(clouds.rgb, 0.0), clouds.a);
#elif (CLOUD_MODE == CLOUD_MODE_NONE)
    vec4 final_color = color;
#endif
    tgt_color = final_color;
}
