#version 330 core
// #extension GL_ARB_texture_storage : require

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

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
// in float f_ao;
// in vec3 f_chunk_pos;
// #ifdef FLUID_MODE_SHINY
flat in uint f_pos_norm;
// #else
// const uint f_pos_norm = 0u;
// #endif
// in float f_alt;
// in vec4 f_shadow;
// in vec3 f_col;
// in float f_light;
/*centroid */in vec2 f_uv_pos;
// in vec3 light_pos[2];
// const vec3 light_pos[6] = vec3[](vec3(0), vec3(0), vec3(00), vec3(0), vec3(0), vec3(0));

/* #if (SHADOW_MODE == SHADOW_MODE_MAP)
in vec4 sun_pos;
#elif (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_NONE)
const vec4 sun_pos = vec4(0.0);
#endif */

uniform sampler2D t_col_light;

layout (std140)
uniform u_locals {
	vec3 model_offs;
	float load_time;
    ivec4 atlas_offs;
};

out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

void main() {
    // discard;
    // vec4 f_col_light = textureGrad(t_col_light, f_uv_pos / texSize, 0.25, 0.25);
    // vec4 f_col_light = texture(t_col_light, (f_uv_pos) / texSize);

	// First 3 normals are negative, next 3 are positive
	const vec3 normals[8] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1), vec3(0,0,0), vec3(0,0,0));

    // uint norm_index = (f_pos_norm >> 29) & 0x7u;
    // vec2 uv_delta = (norm_index & 0u) == 0u ? vec2(-1.0) : vec2(0);

    vec2 f_uv_pos = f_uv_pos + atlas_offs.xy;
    // vec4 f_col_light = textureProj(t_col_light, vec3(f_uv_pos + 0.5, textureSize(t_col_light, 0)));//(f_uv_pos/* + 0.5*/) / texSize);
    // float f_light = textureProj(t_col_light, vec3(f_uv_pos + 0.5, textureSize(t_col_light, 0))).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;
    vec4 f_col_light = texelFetch(t_col_light, ivec2(f_uv_pos)/* + uv_delta*//* - f_norm * 0.00001*/, 0);
    // float f_light = f_col_light.a;
    // vec4 f_col_light = texelFetch(t_col_light, ivec2(int(f_uv_pos.x), int(f_uv_pos.y)/* + uv_delta*//* - f_norm * 0.00001*/), 0);
    vec3 f_col = /*linear_to_srgb*//*srgb_to_linear*/(f_col_light.rgb);
	// vec3 f_col = vec3(1.0);
    float f_light = texture(t_col_light, (f_uv_pos + 0.5) / textureSize(t_col_light, 0)).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;
    // vec2 texSize = textureSize(t_col_light, 0);
    // float f_light = texture(t_col_light, f_uv_pos/* + vec2(atlas_offs.xy)*/).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;
    // float f_light = textureProj(t_col_light, vec3(f_uv_pos/* + vec2(atlas_offs.xy)*/, texSize.x)).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;
    // float f_light = textureProjLod(t_col_light, vec3(f_uv_pos/* + vec2(atlas_offs.xy)*/, texSize.x), 0).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;
    // float f_light = textureGrad(t_col_light, (f_uv_pos + 0.5) / texSize, vec2(0.1, 0.0), vec2(0.0, 0.1)).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;
    // f_light = sqrt(f_light);
    // f_light = sqrt(f_light);
    // f_col = vec3((uvec3(v_col_light) >> uvec3(8, 16, 24)) & uvec3(0xFFu)) / 255.0;
    // vec3 f_col = light_col.rgb;//vec4(1.0, 0.0, 0.0, 1.0);

    // float f_ao = 1.0;

    // vec3 my_chunk_pos = vec3(ivec3((uvec3(f_pos_norm) >> uvec3(0, 6, 12)) & uvec3(0x3Fu, 0x3Fu, 0xFFFFu)));
    // tgt_color = vec4(hash(floor(vec4(my_chunk_pos.x, 0, 0, 0))), hash(floor(vec4(0, my_chunk_pos.y, 0, 1))), hash(floor(vec4(0, 0, my_chunk_pos.z, 2))), 1.0);
    // tgt_color.rgb *= f_light;
    // tgt_color = vec4(vec3(f_light), 1.0);
    // tgt_color = vec4(f_col, 1.0);
    // return;
    // vec4 light_pos[2];
    // vec4 light_col = vec4(
    //          hash(floor(vec4(f_pos.x, 0, 0, 0))),
    //          hash(floor(vec4(0, f_pos.y, 0, 1))),
    //          hash(floor(vec4(0, 0, f_pos.z, 2))),
    //          1.0
    //     );
    // vec3 f_col = light_col.rgb;//vec4(1.0, 0.0, 0.0, 1.0);
    // tgt_color = vec4(f_col, 1.0);
    // tgt_color = vec4(light_shadow_count.x <= 31u ? f_col : vec3(0.0), 1.0);
    // tgt_color = vec4(0.0, 0.0, 0.0, 1.0);
    // float sum = 0.0;
	// for (uint i = 0u; i < /* 6 * */light_shadow_count.x; i ++) {
    //     // uint i = 1u;
	// 	Light L = lights[i/* / 6*/];

    //     /* vec4 light_col = vec4(
    //         hash(vec4(1.0, 0.0, 0.0, i)),
    //         hash(vec4(1.0, 1.0, 0.0, i)),
    //         hash(vec4(1.0, 0.0, 1.0, i)),
    //         1.0
    //     ); */
    //     vec3 light_col = vec3(1.0);//L.light_col.rgb;
    //     float light_strength = L.light_col.a / 255.0;
    //     // float light_strength = 1.0 / light_shadow_count.x;

	// 	vec3 light_pos = L.light_pos.xyz;

	// 	// Pre-calculate difference between light and fragment
	// 	vec3 fragToLight = f_pos - light_pos;

    //     //  vec3 f_norm = normals[(f_pos_norm >> 29) & 0x7u];

    //     // use the light to fragment vector to sample from the depth map
    //     float bias = 0.0;//0.05;//0.05;
    //     // float closestDepth = texture(t_shadow_maps, vec4(fragToLight, i)/*, 0.0*//*, bias*/).r;
    //     // float closestDepth = texture(t_shadow_maps, vec4(fragToLight, lightIndex), bias);
    //     // float closestDepth = texture(t_shadow_maps, vec4(fragToLight, i + 1)/*, bias*/).r;
    //     float currentDepth = VectorToDepth(fragToLight) + bias;
    //     float closestDepth = texture(t_shadow_maps, vec3(fragToLight)/*, -2.5*/).r;
    //
    //     // float visibility = texture(t_shadow_maps, vec4(fragToLight, i + 1), -(length(fragToLight) - bias)/* / screen_res.w*/);
    //     // it is currently in linear range between [0,1]. Re-transform back to original value
    //     // closestDepth *= screen_res.w; // far plane
    //     // now test for shadows
    //     // float shadow = /*currentDepth*/(screen_res.w - bias) > closestDepth ? 1.0 : 0.0;
    //     // float shadow = currentDepth - bias > closestDepth ? 1.0 : 0.0;

    //     // tgt_color += light_col * vec4(vec3(/*closestDepth*/visibility/* + bias*//* / screen_res.w */) * 1.0 / light_shadow_count.x, 0.0);
    //     // tgt_color.rgb += light_col * vec3(closestDepth + 0.05 / screen_res.w) * 1.0 /*/ light_shadow_count.x*/ * light_strength;
    //     tgt_color.rgb += light_col * vec3(closestDepth) * 1.0 / screen_res.w /*/ light_shadow_count.x*/ * light_strength;
    //     sum += light_strength;
    // }

	// TODO: last 3 bits in v_pos_norm should be a number between 0 and 5, rather than 0-2 and a direction.
	// uint norm_axis = (f_pos_norm >> 30) & 0x3u;
	// // Increase array access by 3 to access positive values
	// uint norm_dir = ((f_pos_norm >> 29) & 0x1u) * 3u;
	// Use an array to avoid conditional branching
    // uint norm_index = (f_pos_norm >> 29) & 0x7u;
	// vec3 f_norm = normals[norm_index];
	vec3 f_norm = normals[(f_pos_norm >> 29) & 0x7u];
    // vec3 du = dFdx(f_pos);
    // vec3 dv = dFdy(f_pos);
    // vec3 f_norm = normalize(cross(du, dv));

    // /* if (light_shadow_count.x == 1) {
    //     tgt_color.rgb = vec3(0.0);
    // } */
    // if (sum > 0.0) {
    //     tgt_color.rgb /= sum;
    // }
    // return;
    // Whether this face is facing fluid or not.
    bool faces_fluid = bool((f_pos_norm >> 28) & 0x1u);

    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    // vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    // vec3 view_dir = normalize(-vec3(vert_pos4)/* / vert_pos4.w*/);
    vec3 view_dir = -cam_to_frag;
    // vec3 view_dir = normalize(f_pos - cam_pos.xyz);

    /* vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x); */

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE == FLUID_MODE_SHINY)
    float f_alt = alt_at(f_pos.xy);
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_CHEAP)
    float f_alt = f_pos.z;
#endif

    float alpha = 1.0;//0.0001;//1.0;
    // TODO: Possibly angle with water surface into account?  Since we can basically assume it's horizontal.
    const float n2 = 1.5;//1.01;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    // float faces_fluid = faces_fluid && f_pos.z <= floor(f_alt);
    float fluid_alt = max(f_pos.z + 1, floor(f_alt));
    float R_s = /*(f_pos.z < f_alt)*/faces_fluid /*&& f_pos.z <= fluid_alt*/ ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    // vec3 surf_color = /*srgb_to_linear*/(f_col);
    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);

    // float sun_light = get_sun_brightness(sun_dir);
	// float moon_light = get_moon_brightness(moon_dir);
    /* float sun_shade_frac = horizon_at(f_pos, sun_dir);
    float moon_shade_frac = horizon_at(f_pos, moon_dir); */
    // float f_alt = alt_at(f_pos.xy);
    // vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP)
    vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#elif (SHADOW_MODE == SHADOW_MODE_NONE)
    float sun_shade_frac = 1.0;//horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#endif
    float moon_shade_frac = 1.0;//horizon_at2(f_shadow, f_alt, f_pos, moon_dir);
    // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-f_norm, sun_dir)) * 10000.0), 0.0, 0.5);
    // NOTE: current assumption is that moon and sun shouldn't be out at the sae time.
    // This assumption is (or can at least easily be) wrong, but if we pretend it's true we avoids having to explicitly pass in a separate shadow
    // for the sun and moon (since they have different brightnesses / colors so the shadows shouldn't attenuate equally).
    // float shade_frac = /*1.0;*/sun_shade_frac + moon_shade_frac;

    // DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, light_pos);
    float point_shadow = shadow_at(f_pos, f_norm);
    DirectionalLight sun_info = get_sun_info(sun_dir, point_shadow * sun_shade_frac, /*sun_pos*/f_pos);
    DirectionalLight moon_info = get_moon_info(moon_dir, point_shadow * moon_shade_frac/*, light_pos*/);

    float max_light = 0.0;

    // After shadows are computed, we use a refracted sun and moon direction.
    // sun_dir = faces_fluid && sun_shade_frac > 0.0 ? refract(sun_dir/*-view_dir*/, vec3(0.0, 0.0, 1.0), 1.0 / 1.3325) : sun_dir;
    // moon_dir = faces_fluid && moon_shade_frac > 0.0 ? refract(moon_dir/*-view_dir*/, vec3(0.0, 0.0, 1.0), 1.0 / 1.3325) : moon_dir;

    // Compute attenuation due to water from the camera.
    vec3 mu = faces_fluid/* && f_pos.z <= fluid_alt*/ ? MU_WATER : vec3(0.0);
    // NOTE: Default intersection point is camera position, meaning if we fail to intersect we assume the whole camera is in water.
    vec3 cam_attenuation =
        medium.x == 1u ? compute_attenuation_point(cam_pos.xyz, view_dir, MU_WATER, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/f_pos)
        : compute_attenuation_point(f_pos, -view_dir, mu, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/cam_pos.xyz);

    // Computing light attenuation from water.
    vec3 emitted_light, reflected_light;
    // To account for prior saturation
    /*float */f_light = faces_fluid ? 1.0 : f_light * sqrt(f_light);

    emitted_light = vec3(1.0);
    reflected_light = vec3(1.0);
    float f_select = (select_pos.w > 0 && select_pos.xyz == floor(f_pos - f_norm * 0.5)) ? 1.0 / PERSISTENT_AMBIANCE : 1.0;
    max_light += get_sun_diffuse2(/*time_of_day.x, */sun_info, moon_info, f_norm, view_dir, f_pos, mu, cam_attenuation, fluid_alt, k_a * f_select/* * (shade_frac * 0.5 + light_frac * 0.5)*/, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    // emitted_light *= f_light * point_shadow * max(shade_frac, MIN_SHADOW);
    // reflected_light *= f_light * point_shadow * shade_frac;
    // max_light *= f_light * point_shadow * shade_frac;
    emitted_light *= f_light;
    reflected_light *= f_light;
    max_light *= f_light;

    max_light += lights_at(f_pos, f_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    // float f_ao = 1.0;

	// float ao = /*pow(f_ao, 0.5)*/f_ao * 0.9 + 0.1;
	// emitted_light *= ao;
	// reflected_light *= ao;
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
    vec3 f_chunk_pos = f_pos - (model_offs - focus_off.xyz);
    float noise = hash(vec4(floor(f_chunk_pos * 3.0 - f_norm * 0.5), 0));//0.005/* - 0.01*/;

//vec3 srgb_to_linear(vec3 srgb) {
//    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
//    vec3 higher = pow((srgb + vec3(0.055))/vec3(1.055), vec3(2.4));
//    vec3 lower = srgb/vec3(12.92);
//
//    return mix(higher, lower, cutoff);
//}
//
//vec3 linear_to_srgb(vec3 col) {
//    // bvec3 cutoff = lessThan(col, vec3(0.0060));
//    // return mix(11.500726 * col, , cutoff);
//    vec3 s1 = vec3(sqrt(col.r), sqrt(col.g), sqrt(col.b));
//    vec3 s2 = vec3(sqrt(s1.r), sqrt(s1.g), sqrt(s1.b));
//    vec3 s3 = vec3(sqrt(s2.r), sqrt(s2.g), sqrt(s2.b));
//    return vec3(
//            mix(11.500726 * col.r, (0.585122381 * s1.r + 0.783140355 * s2.r - 0.368262736 * s3.r), clamp((col.r - 0.0060) * 10000.0, 0.0, 1.0)),
//            mix(11.500726 * col.g, (0.585122381 * s1.g + 0.783140355 * s2.g - 0.368262736 * s3.g), clamp((col.g - 0.0060) * 10000.0, 0.0, 1.0)),
//            mix(11.500726 * col.b, (0.585122381 * s1.b + 0.783140355 * s2.b - 0.368262736 * s3.b), clamp((col.b - 0.0060) * 10000.0, 0.0, 1.0))
//    );
//
//  11.500726
//}
    // vec3 noise_delta = vec3(noise * 0.005);
    // vec3 noise_delta = noise * 0.02 * (1.0 - vec3(0.2126, 0.7152, 0.0722));
    // vec3 noise_delta = noise * 0.002 / vec3(0.2126, 0.7152, 0.0722);
    // vec3 noise_delta = sqrt(f_col) + noise;
    /* vec3 noise_delta = f_col + noise * 0.02;
    noise_delta *= noise_delta;
    noise_delta -= f_col; */
    // vec3 noise_delta = (1.0 - f_col) * 0.02 * noise * noise;
    //
    // a = 0.055
    //
    // 1 / (1 + a) = 1 / (1 + 0.055) ~ 0.947867299
    //
    // l2s = x^(1/2.4) * (1 / (1 + a)) - a + c
    // s2l = (l + a)^2.4 * (1 / (1 + a))^2.4
    //     = ((x^(1/2.4) * (1 / (1 + a)) - a + c) + a)^2.4 * (1 / (1 + a))^2.4
    //     = (x^(1/2.4) * (1 / (1 + a)) + c)^2.4 * (1 / (1 + a))^2.4
    //
    //     ~ (x^(1/2) * 1 / (1 + a) + c)^2 * (1 / (1 + a))^2
    //
    //   = ((x + a)^2.4 * (1 / (1 + a))^2.4 + c)^(1/2.4) * (1 / (1 + a))^(1/2.4)
    //   = (((x + a)^2.4 + c * (1 + a)^2.4) * (1 / (1 + a))^2.4)^(1/2.4) * (1 / (1 + a))^(1/2.4)
    //   = ((x + a)^2.4 + c * (1 + a)^2.4)^(1/2.4) * ((1 / (1 + a))^2.4)^(1/2.4) * (1 / (1 + a))^(1/2.4)
    //   = ((x + a)^2.4 + c * (1 + a)^2.4)^(1/2.4) * (1 / (1 + a))^(1/2.4)
    //
    //   = ((x + a)^2 + c * (1 + a)^2)^(1/2) * (1 / (1 + a))^(1/2)
    //   = (x^2 + a^2 + 2xa + c + ca^2 + 2ac)^(1/2) * (1 / (1 + a))^(1/2)
    //
    const float A = 0.055;
    const float W_INV = 1 / (1 + A);
    const float W_2 = W_INV * W_INV;//pow(W_INV, 2.4);
    const float NOISE_FACTOR = 0.02;//pow(0.02, 1.2);
    vec3 noise_delta = (sqrt(f_col) * W_INV + noise * NOISE_FACTOR);
    // noise_delta = noise_delta * noise_delta * W_2 - f_col;
    // lum = W ⋅ col
    // lum + noise = W ⋅ (col + delta)
    // W ⋅ col + noise = W ⋅ col + W ⋅ delta
    // noise = W ⋅ delta
    // delta = noise / W
    // vec3 col = (f_col + noise_delta);
    vec3 col = noise_delta * noise_delta * W_2;
    // vec3 col = srgb_to_linear(linear_to_srgb(f_col) + noise * 0.02);
	// vec3 col = /*srgb_to_linear*/(f_col + noise); // Small-scale noise
	// vec3 col = /*srgb_to_linear*/(f_col + hash(vec4(floor(f_pos * 3.0 - f_norm * 0.5), 0)) * 0.01); // Small-scale noise
    vec3 surf_color = illuminate(max_light, view_dir, col * emitted_light, col * reflected_light);

#if (CLOUD_MODE == CLOUD_MODE_REGULAR)
    float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag/*view_dir*/, time_of_day.x, cam_pos.xyz, f_pos, 1.0, false, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);
#elif (CLOUD_MODE == CLOUD_MODE_NONE)
    vec3 color = surf_color;
#endif

	tgt_color = vec4(color, 1.0);
}
