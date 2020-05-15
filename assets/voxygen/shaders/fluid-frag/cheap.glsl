#version 400 core

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

#include <globals.glsl>
#include <random.glsl>

in vec3 f_pos;
flat in uint f_pos_norm;
in vec3 f_col;
in float f_light;


layout (std140)
uniform u_locals {
    vec3 model_offs;
	float load_time;
};

uniform sampler2D t_waves;

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
	vec3 f_norm = normals[norm_axis + norm_dir];

    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    // vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    // vec3 view_dir = normalize(-vec3(vert_pos4)/* / vert_pos4.w*/);
    vec3 view_dir = -cam_to_frag;
	// vec3 surf_color = /*srgb_to_linear*/(vec3(0.4, 0.7, 2.0));
    /*const */vec3 water_color = 1.0 - MU_WATER;//srgb_to_linear(vec3(0.2, 0.5, 1.0));
    // /*const */vec3 water_color = srgb_to_linear(vec3(0.0, 0.25, 0.5));

    vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x);
    float f_alt = alt_at(f_pos.xy);
    vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
    float moon_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, moon_dir);
    // float sun_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, sun_dir);
    // float moon_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, moon_dir);
    float shade_frac = /*1.0;*/sun_shade_frac + moon_shade_frac;

    float fluid_alt = f_pos.z;//max(ceil(f_pos.z), floor(f_alt));// f_alt;//max(f_alt - f_pos.z, 0.0);

    const float alpha = 0.255/* / 4.0 / sqrt(2.0)*/;
    const float n2 = 1.3325;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float R_s = (f_pos.z < fluid_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    // Water is transparent so both normals are valid.
    vec3 cam_norm = faceforward(f_norm, f_norm, cam_to_frag);

    vec3 mu = MU_WATER;
    // NOTE: Default intersection point is camera position, meaning if we fail to intersect we assume the whole camera is in water.
    vec3 cam_attenuation = vec3(1.0);//compute_attenuation_point(f_pos, -view_dir, mu, fluid_alt, cam_pos.xyz);

    // NOTE: Assumes normal is vertical.
    vec3 sun_view_dir = cam_pos.z <= fluid_alt ? /*refract(view_dir, -f_norm, 1.0 / n2)*//*reflect(view_dir, -f_norm)*/-view_dir : view_dir;//vec3(view_dir.xy, -view_dir.z) : view_dir;

    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);

    vec3 emitted_light, reflected_light;

	// float point_shadow = shadow_at(f_pos, f_norm);
	// vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	// vec3 emitted_light, reflected_light;
	// vec3 light, diffuse_light, ambient_light;
	float point_shadow = shadow_at(f_pos,f_norm);
    // Squared to account for prior saturation.
    float f_light = pow(f_light, 1.5);
    // float vert_light = f_light;
    // vec3 light_frac = /*vec3(1.0);*/light_reflection_factor(f_norm/*vec3(0, 0, 1.0)*/, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(R_s), alpha);

	// vec3 surf_color = /*srgb_to_linear*/(vec3(0.4, 0.7, 2.0));
    float max_light = 0.0;
    max_light += get_sun_diffuse2(f_norm, /*time_of_day.x*/sun_dir, moon_dir, /*-cam_to_frag*/sun_view_dir/*view_dir*/, f_pos, mu, cam_attenuation, fluid_alt, k_a/* * (shade_frac * 0.5 + light_frac * 0.5)*/, /*vec3(0.0)*/k_d, k_s, alpha, 1.0, emitted_light, reflected_light);
    reflected_light *= f_light * point_shadow * shade_frac;
    emitted_light *= f_light * point_shadow * max(shade_frac, MIN_SHADOW);
    max_light *= f_light * point_shadow * shade_frac;
	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 0.0);
	// diffuse_light *= f_light * point_shadow;
	// ambient_light *= f_light, point_shadow;
	// vec3 point_light = light_at(f_pos, f_norm);
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;
	// vec3 surf_color = srgb_to_linear(vec3(0.4, 0.7, 2.0)) * light * diffuse_light * ambient_light;

    // lights_at(f_pos, f_norm, cam_to_frag, k_a * f_light * point_shadow, k_d * f_light * point_shadow, k_s * f_light * point_shadow, alpha, emitted_light, reflected_light);
    /*vec3 point_light = light_at(f_pos, f_norm);
    emitted_light += point_light;
    reflected_light += point_light; */

    max_light += lights_at(f_pos, /*f_norm*/cam_norm, view_dir, mu, cam_attenuation, fluid_alt,  k_a, k_d, k_s, alpha, 1.0, emitted_light, reflected_light);
    // vec3 diffuse_light_point = vec3(0.0);
    // max_light += lights_at(f_pos, f_norm, view_dir, k_a, vec3(1.0), k_s, alpha, emitted_light, diffuse_light_point);

    float reflected_light_point = length(reflected_light);///*length*/(diffuse_light_point.r) + f_light * point_shadow;
    // vec3 dump_light = vec3(0.0);
    // vec3 specular_light_point = vec3(0.0);
    // lights_at(f_pos, f_norm, view_dir, vec3(0.0), vec3(0.0), /*vec3(1.0)*/k_s, alpha, dump_light, specular_light_point);
    // diffuse_light_point -= specular_light_point;

    // float reflected_light_point = /*length*/(diffuse_light_point.r) + f_light * point_shadow;
    // reflected_light += k_d * (diffuse_light_point + f_light * point_shadow * shade_frac) + specular_light_point;

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
    vec3 fog_color = get_sky_color(cam_to_frag, time_of_day.x, cam_pos.xyz, f_pos, 0.25, true, clouds);

	float passthrough = /*pow(*/dot(cam_norm, -cam_to_frag/*view_dir*/)/*, 0.5)*/;

    vec3 surf_color = illuminate(max_light, view_dir, water_color * fog_color * emitted_light, /*surf_color * */water_color * reflected_light);
	vec4 color = mix(vec4(surf_color, 1.0), vec4(surf_color, 1.0 / (1.0 + /*diffuse_light*//*(f_light * point_shadow + point_light)*/4.0 * reflected_light_point/* * 0.25*/)), passthrough);

    tgt_color = mix(mix(color, vec4(fog_color, 0.0), fog_level), vec4(clouds.rgb, 0.0), clouds.a);
}
