#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_TRANSMISSION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_SPECULAR

#if (FLUID_MODE == FLUID_MODE_CHEAP)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE == FLUID_MODE_SHINY)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <sky.glsl>
#include <lod.glsl>

in vec3 f_pos;

layout (std140)
uniform u_locals {
	vec4 nul;
};

out vec4 tgt_color;

void main() {
    // tgt_color = vec4(MU_SCATTER, 1.0);
    // return;
	vec4 _clouds;

	vec3 cam_dir = normalize(f_pos - cam_pos.xyz);

    float cam_alt = alt_at(cam_pos.xy);
    // float f_alt = alt_at(f_pos.xy);
    float fluid_alt = medium.x == 1u ? floor(cam_alt + 1) : view_distance.w;
    // float fluid_alt = max(f_pos.z + 1, floor(f_alt));
    vec3 mu = medium.x == 1u /* && f_pos.z <= fluid_alt*/ ? MU_WATER : vec3(0.0);
    // vec3 sun_attenuation = compute_attenuation(wpos, -sun_dir, mu, surface_alt, wpos);
    vec3 cam_attenuation = compute_attenuation(cam_pos.xyz, -cam_dir, mu, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*//*f_pos*//*vec3(f_pos.xy, fluid_alt)*/cam_pos.xyz);
    // vec3 cam_attenuation = compute_attenuation_point(f_pos, -view_dir, mu, fluid_alt, cam_pos.xyz);
    // vec3 cam_attenuation = vec3(1.0);


	/* vec3 world_pos = cam_pos.xyz + cam_dir * 500000.0;
	tgt_color = vec4(get_sky_color(normalize(f_pos), time_of_day.x, cam_pos.xyz, world_pos, 1.0, true, _clouds), 1.0); */
	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);

	float dist = 100000.0;

	float refractionIndex = medium.x == 1u ? 1.0 / 1.3325 : 1.0;
	/* if (medium.x == 1u) {
		dist = UNDERWATER_MIST_DIST;
	} */
	vec3 wpos = cam_pos.xyz + /*normalize(f_pos)*/cam_dir * dist;

	tgt_color = vec4(cam_attenuation * get_sky_color(normalize(f_pos), time_of_day.x, cam_pos.xyz, wpos, 1.0, true, refractionIndex, _clouds), 1.0);
}
