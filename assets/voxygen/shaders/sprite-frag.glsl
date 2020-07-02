#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#define HAS_SHADOW_MAPS

#include <globals.glsl>

in vec3 f_pos;
flat in vec3 f_norm;
flat in float f_light;
// flat in vec3 f_pos_norm;
in vec2 f_uv_pos;
// flat in uint f_atlas_pos;
// in vec3 f_col;
// in float f_ao;
// in float f_light;
// in vec4 light_pos[2];

uniform sampler2D t_col_light;

//struct ShadowLocals {
//	mat4 shadowMatrices;
//    mat4 texture_mat;
//};
//
//layout (std140)
//uniform u_light_shadows {
//    ShadowLocals shadowMats[/*MAX_LAYER_FACES*/192];
//};

out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

const float FADE_DIST = 32.0;

void main() {
    /* if (f_uv_pos.x < 757) {
        discard;
    } */
    // vec2 f_uv_pos = vec2(768,1) + 0.5;
    // vec2 f_uv_pos = vec2(760, 380);// + 0.5;
    // vec2 f_uv_pos = vec2((uvec2(f_atlas_pos) >> uvec2(0, 16)) & uvec2(0xFFFFu, 0xFFFFu)) + 0.5;
    /* if (f_uv_pos.x < 757) {
        discard;
    } */
    // vec3 du = dFdx(f_pos);
    // vec3 dv = dFdy(f_pos);
    // vec3 f_norm = normalize(cross(du, dv));

    // vec4 f_col_light = texture(t_col_light, (f_uv_pos + 0.5) / textureSize(t_col_light, 0)/* + uv_delta*//* - f_norm * 0.00001*/);
    // vec4 f_col_light = textureGrad(t_col_light, (f_uv_pos + 0.5) / textureSize(t_col_light, 0), vec2(0.5), vec2(0.5));
    vec4 f_col_light = texelFetch(t_col_light, ivec2(f_uv_pos)/* + uv_delta*//* - f_norm * 0.00001*/, 0);
    vec3 f_col = /*linear_to_srgb*//*srgb_to_linear*/(f_col_light.rgb);
	// vec3 f_col = vec3(1.0);
    // vec2 texSize = textureSize(t_col_light, 0);
    // float f_ao = f_col_light.a;
    // float f_ao = f_col_light.a + length(vec2(dFdx(f_col_light.a), dFdy(f_col_light.a)));
    float f_ao = texture(t_col_light, (f_uv_pos + 0.5) / textureSize(t_col_light, 0)).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;
    // float f_ao = 1.0;
    // float /*f_light*/f_ao = textureProj(t_col_light, vec3(f_uv_pos, texSize)).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;

    // vec3 my_chunk_pos = f_pos_norm;
    // tgt_color = vec4(hash(floor(vec4(my_chunk_pos.x, 0, 0, 0))), hash(floor(vec4(0, my_chunk_pos.y, 0, 1))), hash(floor(vec4(0, 0, my_chunk_pos.z, 2))), 1.0);
	// tgt_color = vec4(f_uv_pos / texSize, 0.0, 1.0);
	// tgt_color = vec4(f_col.rgb, 1.0);
    // return;
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

    /* vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x); */
    // float sun_light = get_sun_brightness(sun_dir);
	// float moon_light = get_moon_brightness(moon_dir);

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE == FLUID_MODE_SHINY)
    float f_alt = alt_at(f_pos.xy);
    // float f_alt = f_pos.z;
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_CHEAP)
    float f_alt = f_pos.z;
#endif

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP)
    vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
    // float sun_shade_frac = 1.0;//horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#elif (SHADOW_MODE == SHADOW_MODE_NONE)
    float sun_shade_frac = 1.0;//horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#endif
    float moon_shade_frac = 1.0;//horizon_at2(f_shadow, f_alt, f_pos, moon_dir);
    // float sun_shade_frac = horizon_at(f_pos, sun_dir);
    // float moon_shade_frac = horizon_at(f_pos, moon_dir);
    // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-f_norm, sun_dir)) * 10000.0), 0.0, 0.5);
    // NOTE: current assumption is that moon and sun shouldn't be out at the sae time.
    // This assumption is (or can at least easily be) wrong, but if we pretend it's true we avoids having to explicitly pass in a separate shadow
    // for the sun and moon (since they have different brightnesses / colors so the shadows shouldn't attenuate equally).
    // float shade_frac = sun_shade_frac + moon_shade_frac;

    // DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, light_pos);
    float point_shadow = shadow_at(f_pos, f_norm);
    DirectionalLight sun_info = get_sun_info(sun_dir, point_shadow * sun_shade_frac, /*sun_pos*/f_pos);
    DirectionalLight moon_info = get_moon_info(moon_dir, point_shadow * moon_shade_frac/*, light_pos*/);

	vec3 surf_color = /*srgb_to_linear*//*linear_to_srgb*/(f_col);
    float alpha = 1.0;
    const float n2 = 1.5;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float R_s = (f_pos.z < f_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);

    vec3 emitted_light, reflected_light;

    // To account for prior saturation.
    // float vert_light = pow(f_light, 1.5);
    // vec3 light_frac = light_reflection_factor(f_norm/*vec3(0, 0, 1.0)*/, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(R_s), alpha);
    /* light_frac += light_reflection_factor(f_norm, view_dir, vec3(1.0, 0, 0.0), vec3(1.0), vec3(1.0), 2.0);
    light_frac += light_reflection_factor(f_norm, view_dir, vec3(-1.0, 0, 0.0), vec3(1.0), vec3(1.0), 2.0);
    light_frac += light_reflection_factor(f_norm, view_dir, vec3(0.0, -1.0, 0.0), vec3(1.0), vec3(1.0), 2.0);
    light_frac += light_reflection_factor(f_norm, view_dir, vec3(0.0, 1.0, 0.0), vec3(1.0), vec3(1.0), 2.0); */

	// vec3 light, diffuse_light, ambient_light;
    // vec3 emitted_light, reflected_light;
	// float point_shadow = shadow_at(f_pos,f_norm);
	// vec3 point_light = light_at(f_pos, f_norm);
	// vec3 surf_color = srgb_to_linear(vec3(0.2, 0.5, 1.0));
    // vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    float max_light = 0.0;
    max_light += get_sun_diffuse2(sun_info, moon_info, f_norm, /*time_of_day.x, *//*cam_to_frag*/view_dir, k_a * f_light/* * (shade_frac * 0.5 + light_frac * 0.5)*/, k_d, k_s, alpha, emitted_light, reflected_light);
    // reflected_light *= /*vert_light * */point_shadow * shade_frac;
    // emitted_light *= /*vert_light * */point_shadow * max(shade_frac, MIN_SHADOW);
    // max_light *= /*vert_light * */point_shadow * shade_frac;
    // emitted_light *= point_shadow;
    // reflected_light *= point_shadow;
    // max_light *= point_shadow;
	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	// float point_shadow = shadow_at(f_pos, f_norm);
	// diffuse_light *= f_light * point_shadow;
	// ambient_light *= f_light * point_shadow;
	// light += point_light;
	// diffuse_light += point_light;
    // reflected_light += point_light;

    max_light += lights_at(f_pos, f_norm, view_dir, k_a, k_d, k_s, alpha, emitted_light, reflected_light);
    /* vec3 point_light = light_at(f_pos, f_norm);
    emitted_light += point_light;
    reflected_light += point_light; */

	// float ao = /*pow(f_ao, 0.5)*/f_ao * 0.85 + 0.15;
    float ao = f_ao;
	emitted_light *= ao;
	reflected_light *= ao;

	surf_color = illuminate(max_light, view_dir, surf_color * emitted_light, surf_color * reflected_light);
	// vec3 surf_color = illuminate(f_col, light, diffuse_light, ambient_light);

#if (CLOUD_MODE == CLOUD_MODE_REGULAR)
	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag/*view_dir*/, time_of_day.x, cam_pos.xyz, f_pos, 0.5, false, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);
#elif (CLOUD_MODE == CLOUD_MODE_NONE)
    vec3 color = surf_color;
#endif

	// tgt_color = vec4(color, 1.0);
	tgt_color = vec4(color, 1.0 - clamp((distance(focus_pos.xy, f_pos.xy) - (sprite_render_distance - FADE_DIST)) / FADE_DIST, 0, 1));
}
