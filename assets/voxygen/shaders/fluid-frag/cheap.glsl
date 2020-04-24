#version 330 core

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
	vec3 normals[6] = vec3[]( vec3(-1,0,0), vec3(0,-1,0), vec3(0,0,-1), vec3(1,0,0), vec3(0,1,0), vec3(0,0,1) );

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
    /*const */vec3 water_color = srgb_to_linear(vec3(0.2, 0.5, 1.0));

    vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x);
    float sun_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, sun_dir);
    float moon_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, moon_dir);
    float shade_frac = /*1.0;*/sun_shade_frac + moon_shade_frac;

    const float alpha = 0.255/* / 4.0 / sqrt(2.0)*/;
    const float n2 = 1.3325;
    const float R_s = pow((1.0 - n2) / (1.0 + n2), 2);

    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);

    vec3 emitted_light, reflected_light;

	// float point_shadow = shadow_at(f_pos, f_norm);
	// vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
	// vec3 emitted_light, reflected_light;
	// vec3 light, diffuse_light, ambient_light;
	float point_shadow = shadow_at(f_pos,f_norm);
    // float vert_light = f_light;
    // vec3 light_frac = /*vec3(1.0);*/light_reflection_factor(f_norm/*vec3(0, 0, 1.0)*/, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(R_s), alpha);

	// vec3 surf_color = /*srgb_to_linear*/(vec3(0.4, 0.7, 2.0));
    get_sun_diffuse2(f_norm, /*time_of_day.x*/sun_dir, moon_dir, /*-cam_to_frag*/view_dir, k_a/* * (shade_frac * 0.5 + light_frac * 0.5)*/, vec3(0.0), k_s, alpha, emitted_light, reflected_light);
    reflected_light *= f_light * point_shadow * shade_frac;
    emitted_light *= f_light * point_shadow;
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

    vec3 diffuse_light_point = vec3(0.0);
    lights_at(f_pos, f_norm, view_dir, k_a, vec3(1.0), vec3(k_s), alpha, emitted_light, diffuse_light_point);

    vec3 dump_light = vec3(0.0);
    vec3 specular_light_point = vec3(0.0);
    lights_at(f_pos, f_norm, view_dir, vec3(0.0), vec3(0.0), /*vec3(1.0)*/k_s, alpha, dump_light, specular_light_point);
    diffuse_light_point -= specular_light_point;

    float reflected_light_point = length(diffuse_light_point) + f_light * point_shadow;
    reflected_light += water_color * k_d * (diffuse_light_point + f_light * point_shadow * shade_frac) + k_s * specular_light_point;

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
    vec3 fog_color = get_sky_color(cam_to_frag, time_of_day.x, cam_pos.xyz, f_pos, 0.25, true, clouds);

	float passthrough = pow(dot(faceforward(f_norm, f_norm, cam_to_frag/*view_dir*/), -cam_to_frag/*view_dir*/), 0.5);

    vec3 surf_color = illuminate(water_color * emitted_light, /*surf_color * */reflected_light);
	vec4 color = mix(vec4(surf_color, 1.0), vec4(surf_color, 1.0 / (1.0 + /*diffuse_light*//*(f_light * point_shadow + point_light)*/reflected_light_point/* * 0.25*/)), passthrough);

    tgt_color = mix(mix(color, vec4(fog_color, 0.0), fog_level), vec4(clouds.rgb, 0.0), clouds.a);
}
