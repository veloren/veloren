#version 420 core

#define FIGURE_SHADER

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#if (FLUID_MODE == FLUID_MODE_LOW)
    #define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE >= FLUID_MODE_MEDIUM)
    #define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#define HAS_SHADOW_MAPS

#include <globals.glsl>
#include <light.glsl>
#include <cloud.glsl>
#include <lod.glsl>

layout(location = 0) in vec3 f_pos;
// in float dummy;
// in vec3 f_col;
// in float f_ao;
// flat in uint f_pos_norm;
layout(location = 1) flat in vec3 f_norm;
/*centroid */layout(location = 2) in vec2 f_uv_pos;
layout(location = 3) in vec3 m_pos;
layout(location = 4) in float scale;
// in float f_alt;
// in vec4 f_shadow;
// in vec3 light_pos[2];

// #if (SHADOW_MODE == SHADOW_MODE_MAP)
// in vec4 sun_pos;
// #elif (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_NONE)
// const vec4 sun_pos = vec4(0.0);
// #endif

layout(set = 2, binding = 0)
uniform texture2D t_col_light;
layout(set = 2, binding = 1)
uniform sampler s_col_light;

//struct ShadowLocals {
//  mat4 shadowMatrices;
//    mat4 texture_mat;
//};
//
//layout (std140)
//uniform u_light_shadows {
//    ShadowLocals shadowMats[/*MAX_LAYER_FACES*/192];
//};

layout (std140, set = 3, binding = 0)
uniform u_locals {
    mat4 model_mat;
    vec4 highlight_col;
    vec4 model_light;
    vec4 model_glow;
    ivec4 atlas_offs;
    vec3 model_pos;
    // bit 0 - is player
    // bit 1-31 - unused
    int flags;
};

struct BoneData {
    mat4 bone_mat;
    mat4 normals_mat;
};

layout (std140, set = 3, binding = 1)
uniform u_bones {
    BoneData bones[16];
};

layout(location = 0) out vec4 tgt_color;
layout(location = 1) out uvec4 tgt_mat;

void main() {
    // vec2 texSize = textureSize(t_col_light, 0);
    // vec4 col_light = texture(t_col_light, (f_uv_pos + 0.5) / texSize);
    // vec3 f_col = col_light.rgb;
    // float f_ao = col_light.a;

    // vec4 f_col_light = texture(t_col_light, (f_uv_pos + 0.5) / textureSize(t_col_light, 0));
    // vec3 f_col = f_col_light.rgb;
    // float f_ao = f_col_light.a;

    float f_ao;
    uint material = 0xFFu;
    vec3 f_col = greedy_extract_col_light_figure(t_col_light, s_col_light, f_uv_pos, f_ao, material);

    #ifdef EXPERIMENTAL_BAREMINIMUM
        tgt_color = vec4(simple_lighting(f_pos.xyz, f_col, f_ao), 1);
        return;
    #endif

    // float /*f_light*/f_ao = textureProj(t_col_light, vec3(f_uv_pos, texSize)).a;//1.0;//f_col_light.a * 4.0;// f_light = float(v_col_light & 0x3Fu) / 64.0;

    // vec3 my_chunk_pos = (vec3((uvec3(f_pos_norm) >> uvec3(0, 9, 18)) & uvec3(0x1FFu)) - 256.0) / 2.0;
    // tgt_color = vec4(hash(floor(vec4(my_chunk_pos.x, 0, 0, 0))), hash(floor(vec4(0, my_chunk_pos.y, 0, 1))), hash(floor(vec4(0, 0, my_chunk_pos.z, 2))), 1.0);
    // float f_ao = 0;
    // tgt_color = vec4(vec3(f_ao), 1.0);
    // tgt_color = vec4(f_col, 1.0);
    // return;

    // vec3 du = dFdx(f_pos);
    // vec3 dv = dFdy(f_pos);
    // vec3 f_norm = normalize(cross(du, dv));

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
    /* float sun_shade_frac = horizon_at(f_pos, sun_dir);
    float moon_shade_frac = horizon_at(f_pos, moon_dir); */
#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE >= FLUID_MODE_MEDIUM)
    float f_alt = alt_at(f_pos.xy);
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_LOW)
    float f_alt = f_pos.z;
#endif

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP)
    vec4 f_shadow = textureBicubic(t_horizon, s_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#elif (SHADOW_MODE == SHADOW_MODE_NONE)
    float sun_shade_frac = 1.0;//horizon_at2(f_shadow, f_alt, f_pos, sun_dir);
#endif
    float moon_shade_frac = 1.0;// horizon_at2(f_shadow, f_alt, f_pos, moon_dir);
    // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-f_norm, sun_dir)) * 10000.0), 0.0, 0.5);
    // NOTE: current assumption is that moon and sun shouldn't be out at the sae time.
    // This assumption is (or can at least easily be) wrong, but if we pretend it's true we avoids having to explicitly pass in a separate shadow
    // for the sun and moon (since they have different brightnesses / colors so the shadows shouldn't attenuate equally).
    // float shade_frac = /*1.0;*/sun_shade_frac + moon_shade_frac;

    // DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, light_pos);
    DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, /*sun_pos*/f_pos);
    DirectionalLight moon_info = get_moon_info(moon_dir, moon_shade_frac/*, light_pos*/);

    vec3 surf_color;
    // If the figure is large enough to be 'terrain-like', we apply a noise effect to it
    #ifndef EXPERIMENTAL_NONOISE
        if (scale >= 0.5) {
            float noise = hash(vec4(floor(m_pos * 3.0 - f_norm * 0.5), 0));

            const float A = 0.055;
            const float W_INV = 1 / (1 + A);
            const float W_2 = W_INV * W_INV;
            const float NOISE_FACTOR = 0.015;
            vec3 noise_delta = (sqrt(f_col) * W_INV + noise * NOISE_FACTOR);
            surf_color = noise_delta * noise_delta * W_2;
        } else
    #endif
    {
        surf_color = f_col;
    }

    float alpha = 1.0;
    const float n2 = 1.5;


    // This is a silly hack. It's not true reflectance (see below for that), but gives the desired
    // effect without breaking the entire lighting model until we come up with a better way of doing
    // reflectivity that accounts for physical surroundings like the ground
    if ((material & (1u << 1u)) > 0u) {
        vec3 reflect_ray_dir = reflect(cam_to_frag, f_norm);
        surf_color *= dot(vec3(1.0) - abs(fract(reflect_ray_dir * 1.5) * 2.0 - 1.0) * 0.85, vec3(1));
        alpha = 0.1;
    }

    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float R_s = (f_pos.z < f_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);

    vec3 emitted_light, reflected_light;

    // Make voxel shadows block the sun and moon
    sun_info.block *= model_light.x;
    moon_info.block *= model_light.x;

    // vec3 light_frac = /*vec3(1.0);*//*vec3(max(dot(f_norm, -sun_dir) * 0.5 + 0.5, 0.0));*/light_reflection_factor(f_norm, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(R_s), alpha);
    // vec3 point_light = light_at(f_pos, f_norm);
    // vec3 light, diffuse_light, ambient_light;
    //get_sun_diffuse(f_norm, time_of_day.x, view_dir, k_a * point_shadow * (shade_frac * 0.5 + light_frac * 0.5), k_d * point_shadow * shade_frac, k_s * point_shadow * shade_frac, alpha, emitted_light, reflected_light);
    float max_light = 0.0;
    // reflected_light *= point_shadow * shade_frac;
    // emitted_light *= point_shadow * max(shade_frac, MIN_SHADOW);
    // max_light *= point_shadow * shade_frac;
    // reflected_light *= point_shadow;
    // emitted_light *= point_shadow;
    // max_light *= point_shadow;

    vec3 cam_attenuation = vec3(1);
    float fluid_alt = max(f_pos.z + 1, floor(f_alt + 1));
    vec3 mu = medium.x == MEDIUM_WATER ? MU_WATER : vec3(0.0);
    #if (FLUID_MODE >= FLUID_MODE_MEDIUM)
        cam_attenuation =
            medium.x == MEDIUM_WATER ? compute_attenuation_point(cam_pos.xyz, view_dir, mu, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/f_pos)
            : compute_attenuation_point(f_pos, -view_dir, mu, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/cam_pos.xyz);
    #endif

    // Prevent the sky affecting light when underground
    float not_underground = clamp((f_pos.z - f_alt) / 128.0 + 1.0, 0.0, 1.0);

    max_light += get_sun_diffuse2(sun_info, moon_info, f_norm, view_dir, f_pos, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    max_light += lights_at(f_pos, f_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    // TODO: Hack to add a small amount of underground ambient light to the scene
    reflected_light += vec3(0.01, 0.02, 0.03) * (1.0 - not_underground);

    // Apply baked lighting from emissive blocks
    float glow_mag = length(model_glow.xyz);
    vec3 glow = pow(model_glow.w, 2) * 4
        * glow_light(f_pos)
        * (max(dot(f_norm, model_glow.xyz / glow_mag) * 0.5 + 0.5, 0.0) + max(1.0 - glow_mag, 0.0));
    emitted_light += glow * cam_attenuation;

    // Apply baked AO
    float ao = f_ao * sqrt(f_ao);//0.25 + f_ao * 0.75; ///*pow(f_ao, 0.5)*/f_ao * 0.85 + 0.15;
    reflected_light *= ao;
    emitted_light *= ao;

    // Apply point light AO
    float point_shadow = shadow_at(f_pos, f_norm);
    reflected_light *= point_shadow;
    emitted_light *= point_shadow;

    // Apply emissive glow
    // For now, just make glowing material light be the same colour as the surface
    // TODO: Add a way to control this better outside the shaders
    if ((material & (1u << 0u)) > 0u) {
        emitted_light += 20 * surf_color;
    }

    /* reflected_light *= cloud_shadow(f_pos); */
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
    // vec3 surf_color = illuminate(srgb_to_linear(highlight_col.rgb * f_col), light, diffuse_light, ambient_light);

    float reflectance = 0.0;
    // TODO: Do reflectance properly like this later
    vec3 reflect_color = vec3(0);
    /*
    if ((material & (1u << 1u)) > 0u && false) {
        vec3 reflect_ray_dir = reflect(cam_to_frag, f_norm);
        reflect_color = get_sky_color(reflect_ray_dir, time_of_day.x, f_pos, vec3(-100000), 0.125, true);
        reflect_color = get_cloud_color(reflect_color, reflect_ray_dir, cam_pos.xyz, time_of_day.x, 100000.0, 0.25);
        reflectance = 1.0;
    }
    */

    surf_color = illuminate(max_light, view_dir, mix(surf_color * emitted_light, reflect_color, reflectance), mix(surf_color * reflected_light, reflect_color, reflectance)) * highlight_col.rgb;

    // if ((flags & 1) == 1 && int(cam_mode) == 1) {
    //  float distance = distance(vec3(cam_pos), focus_pos.xyz) - 2;

    //  float opacity = clamp(distance / distance_divider, 0, 1);

    //  // if(threshold_matrix[int(gl_FragCoord.x) % 4][int(gl_FragCoord.y) % 4] > opacity) {
    //     //     discard;
    //     //     return;
    //  // }
    // }

    tgt_color = vec4(surf_color, 1.0);
    tgt_mat = uvec4(uvec3((f_norm + 1.0) * 127.0), MAT_FIGURE);
}
