#version 420 core
// #extension GL_ARB_texture_storage : require

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
#include <random.glsl>

layout(location = 0) in vec3 f_pos;
// in float f_ao;
// in vec3 f_chunk_pos;
// #ifdef FLUID_MODE_SHINY
layout(location = 1) flat in uint f_pos_norm;
layout(location = 2) flat in float f_load_time;
// #else
// const uint f_pos_norm = 0u;
// #endif
// in float f_alt;
// in vec4 f_shadow;
// in vec3 f_col;
// in float f_light;
/*centroid */layout(location = 3) in vec2 f_uv_pos;
// in vec3 light_pos[2];
// const vec3 light_pos[6] = vec3[](vec3(0), vec3(0), vec3(00), vec3(0), vec3(0), vec3(0));

/* #if (SHADOW_MODE == SHADOW_MODE_MAP)
in vec4 sun_pos;
#elif (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_NONE)
const vec4 sun_pos = vec4(0.0);
#endif */

layout(set = 2, binding = 0)
uniform texture2D t_col_light;
layout(set = 2, binding = 1)
uniform sampler s_col_light;

layout (std140, set = 3, binding = 0)
uniform u_locals {
    vec3 model_offs;
    float load_time;
    ivec4 atlas_offs;
};

layout(location = 0) out vec4 tgt_color;
layout(location = 1) out uvec4 tgt_mat;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

void main() {
    /*
    float nz = abs(hash(vec4(floor((f_pos + focus_off.xyz) * 5.0), 0)));
    if (nz > (tick.x - load_time) / 0.5 || distance(focus_pos.xy, f_pos.xy) / view_distance.x + nz * 0.1 > 1.0) {
        discard;
    }
    */

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
    float f_light, f_glow, f_ao, f_sky_exposure;
    vec3 f_col = greedy_extract_col_light_terrain(t_col_light, s_col_light, f_uv_pos, f_light, f_glow, f_ao, f_sky_exposure);

    #ifdef EXPERIMENTAL_BAREMINIMUM
        tgt_color = vec4(simple_lighting(f_pos.xyz, f_col, f_light), 1);
        return;
    #endif

    //float f_light = (uint(texture(t_col_light, (f_uv_pos + 0.5) / textureSize(t_col_light, 0)).r * 255.0) & 0x1Fu) / 31.0;
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
    //     Light L = lights[i/* / 6*/];

    //     /* vec4 light_col = vec4(
    //         hash(vec4(1.0, 0.0, 0.0, i)),
    //         hash(vec4(1.0, 1.0, 0.0, i)),
    //         hash(vec4(1.0, 0.0, 1.0, i)),
    //         1.0
    //     ); */
    //     vec3 light_col = vec3(1.0);//L.light_col.rgb;
    //     float light_strength = L.light_col.a / 255.0;
    //     // float light_strength = 1.0 / light_shadow_count.x;

    //     vec3 light_pos = L.light_pos.xyz;

    //     // Pre-calculate difference between light and fragment
    //     vec3 fragToLight = f_pos - light_pos;

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
    vec3 face_norm = normals[(f_pos_norm >> 29) & 0x7u];
    vec3 f_norm = face_norm;

    #ifdef EXPERIMENTAL_BRICKLOREN
        vec3 pos = f_pos + focus_off.xyz;
        const vec3 bk_sz = vec3(2, 2, 2);
        vec3 sz = vec3(1.0 + mod(floor(pos.z * bk_sz.z + floor(pos.x) + floor(pos.y) - 0.01), 2.0) * (bk_sz.x - 1), 1.0 + mod(floor(pos.z * bk_sz.z + floor(pos.x) + floor(pos.y) + 0.99), 2.0) * (bk_sz.y - 1), bk_sz.z);
        vec3 fp = pos * sz;
        vec3 clamped = min(floor(fp.xyz) + 1.0 - 0.07 * sz, max(floor(fp.xyz) - 0.07 * sz, fp.xyz));
        f_norm.xyz += (fp.xyz - clamped) * 5.0 * sign(1.0 - f_norm) * max(1.0 - length(f_pos - cam_pos.xyz) / 64.0, 0);
        f_norm = normalize(f_norm);
        f_col /= 1.0 + length((fp - clamped) * sign(1.0 - f_norm)) * 2;
    #endif

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

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE >= FLUID_MODE_MEDIUM)
    float f_alt = alt_at(f_pos.xy);
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_LOW)
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
    float fluid_alt = max(f_pos.z + 1, floor(f_alt + 1));
    float R_s = /*(f_pos.z < f_alt)*/faces_fluid /*&& f_pos.z <= fluid_alt*/ ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    // vec3 surf_color = /*srgb_to_linear*/(f_col);
    vec3 k_a = vec3(1.0);
    vec3 k_d = vec3(1.0);
    vec3 k_s = vec3(R_s);

    // Toggle to see rain_occlusion
    // tgt_color = vec4(rain_occlusion_at(f_pos.xyz), 0.0, 0.0, 1.0);
    // return;
    #if (REFLECTION_MODE >= REFLECTION_MODE_HIGH)
        float f_alpha = 1.0;
    #else
        const float f_alpha = 1.0;
    #endif
    #if (CLOUD_MODE != CLOUD_MODE_NONE)
        if (rain_density > 0 && !faces_fluid && f_norm.z > 0.5) {
            vec3 pos = f_pos + focus_off.xyz;
            vec3 drop_density = vec3(2, 2, 2);
            vec3 drop_pos = pos + vec3(pos.zz, 0) + vec3(0, 0, -tick.x * 1.0);
            drop_pos.z += noise_2d(floor(drop_pos.xy * drop_density.xy) * 13.1) * 10;
            vec2 cell2d = floor(drop_pos.xy * drop_density.xy);
            drop_pos.z *= 0.5 + hash_fast(uvec3(cell2d, 0));
            vec3 cell = vec3(cell2d, floor(drop_pos.z * drop_density.z));

            #if (REFLECTION_MODE >= REFLECTION_MODE_HIGH)
                float puddle = clamp((noise_2d((f_pos.xy + focus_off.xy + vec2(0.1, 0)) * 0.02) - 0.5) * 20.0, 0.0, 1.0)
                    * min(rain_density * 10.0, 1.0)
                    * clamp((f_sky_exposure - 0.95) * 50.0, 0.0, 1.0);
            #else
                const float puddle = 1.0;
            #endif

            #if (REFLECTION_MODE >= REFLECTION_MODE_HIGH)
                if (puddle > 0.0) {
                    f_alpha = puddle * 0.2 * max(1.0 + cam_to_frag.z, 0.3);
                    #ifdef EXPERIMENTAL_PUDDLEDETAILS
                        float h = (noise_2d((f_pos.xy + focus_off.xy) * 0.3) - 0.5) * sin(tick.x * 8.0 + f_pos.x * 3)
                            + (noise_2d((f_pos.xy + focus_off.xy) * 0.6) - 0.5) * sin(tick.x * 3.5 - f_pos.y * 6);
                        float hx = (noise_2d((f_pos.xy + focus_off.xy + vec2(0.1, 0)) * 0.3) - 0.5) * sin(tick.x * 8.0 + f_pos.x * 3)
                            + (noise_2d((f_pos.xy + focus_off.xy + vec2(0.1, 0)) * 0.6) - 0.5) * sin(tick.x * 3.5 - f_pos.y * 6);
                        float hy = (noise_2d((f_pos.xy + focus_off.xy + vec2(0, 0.1)) * 0.3) - 0.5) * sin(tick.x * 8.0 + f_pos.x * 3)
                            + (noise_2d((f_pos.xy + focus_off.xy + vec2(0, 0.1)) * 0.6) - 0.5) * sin(tick.x * 3.5 - f_pos.y * 6);
                        f_norm.xy += mix(vec2(0), vec2(h - hx, h - hy) / 0.1 * 0.03, puddle);
                    #endif
                    alpha = mix(1.0, 0.2, puddle);
                    f_col.rgb *= mix(1.0, 0.7, puddle);
                    k_s = mix(k_s, vec3(0.7, 0.7, 1.0), puddle);
                }
            #endif

            if (rain_occlusion_at(f_pos.xyz + vec3(0, 0, 0.25)) > 0.5) {
                if (fract(hash(fract(vec4(cell, 0) * 0.01))) < rain_density * 2.0) {
                    vec3 off = vec3(hash_fast(uvec3(cell * 13)), hash_fast(uvec3(cell * 5)), 0);
                    vec3 near_cell = (cell + 0.5 + (off - 0.5) * 0.5) / drop_density;

                    float dist = length((drop_pos - near_cell) * vec3(1, 1, 0.5));
                    float drop_rad = 0.075 + puddle * 0.05;
                    float distort = max(1.0 - abs(dist - drop_rad) * 100, 0) * 1.5 * max(drop_pos.z - near_cell.z, 0);
                    k_a += distort;
                    k_d += distort;
                    k_s += distort;

                    f_norm.xy += (drop_pos - near_cell).xy
                        * max(1.0 - abs(dist - drop_rad) * 30, 0)
                        * 500.0
                        * max(drop_pos.z - near_cell.z, 0)
                        * sign(dist - drop_rad)
                        * max(drop_pos.z - near_cell.z, 0);
                }
            }
        }
    #endif

    // float sun_light = get_sun_brightness(sun_dir);
    // float moon_light = get_moon_brightness(moon_dir);
    /* float sun_shade_frac = horizon_at(f_pos, sun_dir);
    float moon_shade_frac = horizon_at(f_pos, moon_dir); */
    // float f_alt = alt_at(f_pos.xy);
    // vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP)
    vec4 f_shadow = textureBicubic(t_horizon, s_horizon, pos_to_tex(f_pos.xy));
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
    DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, /*sun_pos*/f_pos);
    DirectionalLight moon_info = get_moon_info(moon_dir, moon_shade_frac/*, light_pos*/);

    #ifdef EXPERIMENTAL_DIRECTIONALSHADOWMAPTEXELGRID
        float offset_scale = 0.5;
        vec3 offset_one = dFdx(f_pos) * offset_scale;
        vec3 offset_two = dFdy(f_pos) * offset_scale;
        vec3 one_up = f_pos + offset_one;
        vec3 one_down = f_pos - offset_one;
        vec3 two_up = f_pos + offset_two;
        vec3 two_down = f_pos - offset_two;

        // Adjust this to change the size of the grid cells relative to the
        // number of shadow texels
        float grid_cell_to_texel_ratio = 32.0;

        vec2 shadowTexSize = textureSize(sampler2D(t_directed_shadow_maps, s_directed_shadow_maps), 0) / grid_cell_to_texel_ratio;

        vec4 one_up_shadow_tex = texture_mat * vec4(one_up, 1.0);
        vec2 oust_snap = floor(one_up_shadow_tex.xy * shadowTexSize / one_up_shadow_tex.w);
        vec4 one_down_shadow_tex = texture_mat * vec4(one_down, 1.0);
        vec2 odst_snap = floor(one_down_shadow_tex.xy * shadowTexSize / one_down_shadow_tex.w);
        vec4 two_up_shadow_tex = texture_mat * vec4(two_up, 1.0);
        vec2 tust_snap = floor(two_up_shadow_tex.xy * shadowTexSize / two_up_shadow_tex.w);
        vec4 two_down_shadow_tex = texture_mat * vec4(two_down, 1.0);
        vec2 tdst_snap = floor(two_down_shadow_tex.xy * shadowTexSize / two_down_shadow_tex.w);
        float border = length(max(abs(oust_snap - odst_snap), abs(tust_snap - tdst_snap)));

        if (border != 0.0) {
            tgt_color = vec4(vec3(0.0, 0.7, 0.2), 1.0);
            return;
        }
    #endif

    float max_light = 0.0;

    // After shadows are computed, we use a refracted sun and moon direction.
    // sun_dir = faces_fluid && sun_shade_frac > 0.0 ? refract(sun_dir/*-view_dir*/, vec3(0.0, 0.0, 1.0), 1.0 / 1.3325) : sun_dir;
    // moon_dir = faces_fluid && moon_shade_frac > 0.0 ? refract(moon_dir/*-view_dir*/, vec3(0.0, 0.0, 1.0), 1.0 / 1.3325) : moon_dir;

    // Compute attenuation due to water from the camera.
    vec3 mu = faces_fluid/* && f_pos.z <= fluid_alt*/ ? MU_WATER : vec3(0.0);
    // NOTE: Default intersection point is camera position, meaning if we fail to intersect we assume the whole camera is in water.
    // Computing light attenuation from water.
    vec3 cam_attenuation =
        false/*medium.x == MEDIUM_WATER*/ ? compute_attenuation_point(cam_pos.xyz, view_dir, MU_WATER, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/f_pos)
        : compute_attenuation_point(f_pos, -view_dir, mu, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/cam_pos.xyz);

    // Prevent the sky affecting light when underground
    float not_underground = clamp((f_pos.z - f_alt) / 128.0 + 1.0, 0.0, 1.0);

    // To account for prior saturation
    #if (FLUID_MODE == FLUID_MODE_LOW)
        f_light = f_light * sqrt(f_light);
    #else
        f_light = faces_fluid ? not_underground : f_light * sqrt(f_light);
    #endif

    vec3 emitted_light = vec3(1.0);
    vec3 reflected_light = vec3(1.0);

    float sun_diffuse = get_sun_diffuse2(/*time_of_day.x, */sun_info, moon_info, f_norm, view_dir, f_pos, mu, cam_attenuation, fluid_alt, k_a/* * (shade_frac * 0.5 + light_frac * 0.5)*/, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);
    max_light += sun_diffuse;

    // emitted_light *= f_light * point_shadow * max(shade_frac, MIN_SHADOW);
    // reflected_light *= f_light * point_shadow * shade_frac;
    // max_light *= f_light * point_shadow * shade_frac;
    emitted_light *= f_light;
    reflected_light *= f_light;
    max_light *= f_light;

    // TODO: Hack to add a small amount of underground ambient light to the scene
    reflected_light += vec3(0.01, 0.02, 0.03) * (1.0 - not_underground);

    // TODO: Apply AO after this
    vec3 glow = glow_light(f_pos) * (pow(f_glow, 3) * 5 + pow(f_glow, 2.0) * 2) * pow(max(dot(face_norm, f_norm), 0), 2);
    reflected_light += glow * cam_attenuation;

    max_light += lights_at(f_pos, f_norm, view_dir, mu, cam_attenuation, fluid_alt, k_a, k_d, k_s, alpha, f_norm, 1.0, emitted_light, reflected_light);

    emitted_light *= mix(1.0, f_ao, 0.5);
    reflected_light *= mix(1.0, f_ao, 0.5);

    float point_shadow = shadow_at(f_pos, f_norm);
    reflected_light *= point_shadow;
    emitted_light *= point_shadow;

    #ifndef EXPERIMENTAL_NOCAUSTICS
        #if (FLUID_MODE >= FLUID_MODE_MEDIUM)
            if (faces_fluid) {
                vec3 wpos = f_pos + vec3(focus_off.xy, 0);
                vec3 spos = (wpos + (fluid_alt - wpos.z) * vec3(sun_dir.xy, 0)) * 0.25;
                reflected_light += caustics(spos.xy * 1.0, tick.x * 0.5)
                    * 3
                    / (1.0 + pow(abs(fluid_alt - wpos.z) * 0.075, 2))
                    * cam_attenuation
                    * max(dot(f_norm, -sun_dir.xyz), 0)
                    * sun_diffuse
                    * sun_info.shadow
                    * f_light;
            }
        #endif
    #endif

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
    #ifdef EXPERIMENTAL_NONOISE
        float noise = 0.0;
    #else
        #ifdef EXPERIMENTAL_BRICKLOREN
            float noise = hash(vec4(floor(clamped), 0)) * 2 + hash(vec4(floor(clamped * 27 / sz), 0)) * 0.5;
        #else
            float noise = hash(vec4(floor(f_chunk_pos * 3.0 - f_norm * 0.5), 0));//0.005/* - 0.01*/;
        #endif
    #endif

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
    const float NOISE_FACTOR = 0.015;//pow(0.02, 1.2);
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

    float f_select = (select_pos.w > 0 && select_pos.xyz == floor(f_pos - f_norm * 0.5)) ? 1.0 : 0.0;
    surf_color += f_select * (surf_color + 0.1) * vec3(0.5, 0.5, 0.5);

    tgt_color = vec4(surf_color, f_alpha);
    tgt_mat = uvec4(uvec3((f_norm + 1.0) * 127.0), MAT_BLOCK);
    //tgt_color = vec4(f_norm, f_alpha);
}
