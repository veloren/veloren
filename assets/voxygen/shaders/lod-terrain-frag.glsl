#version 330 core

#include <globals.glsl>
#include <sky.glsl>
#include <lod.glsl>

in vec3 f_pos;
in vec3 f_norm;
in vec2 v_pos_orig;
// in vec4 f_shadow;
// in vec4 f_square;

out vec4 tgt_color;

#include <sky.glsl>

void main() {
    // vec3 f_pos = lod_pos(f_pos.xy);
	// vec3 f_col = lod_col(f_pos.xy);

    // vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    // vec3 view_dir = normalize(-vec3(vert_pos4)/* / vert_pos4.w*/);

    float my_alt = /*f_pos.z;*/alt_at_real(f_pos.xy);
    // vec3 f_pos = vec3(f_pos.xy, max(my_alt, f_pos.z));
	/* gl_Position =
		proj_mat *
		view_mat *
		vec4(f_pos, 1);
	gl_Position.z = -1000.0 / (gl_Position.z + 10000.0); */
    vec3 my_pos = vec3(f_pos.xy, my_alt);
	vec3 my_norm = lod_norm(f_pos.xy/*, f_square*/);

    float which_norm = dot(my_norm, normalize(cam_pos.xyz - my_pos));
    // which_norm = 0.5 + which_norm * 0.5;
    which_norm = pow(max(0.0, which_norm), /*0.03125*/1 / 8.0);// * 0.5;
    // which_norm = mix(0.0, 1.0, which_norm > 0.0);
	// vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));
    vec3 f_norm = mix(faceforward(f_norm, cam_pos.xyz - f_pos, -f_norm), my_norm, which_norm);
    vec3 f_pos = mix(f_pos, my_pos, which_norm);
    // vec3 fract_pos = fract(f_pos);
    /* if (length(f_pos - cam_pos.xyz) <= view_distance.x + 32.0) {
        vec4 new_f_pos;
        float depth = 10000000.0;
        vec4 old_coord = all_mat * vec4(f_pos.xyz, 1.0);
        for (int i = 0; i < 6; i ++) {
            // vec4 square = focus_pos.xy + vec4(splay(pos - vec2(1.0, 1.0), splay(pos + vec2(1.0, 1.0))));
            vec3 my_f_norm = normals[i];
            vec3 my_f_tan = normals[(i + 2) % 6];
            vec3 my_f_bitan = normals[(i + 4) % 6];
            mat4 foo = mat4(vec4(my_f_tan, 0), vec4(my_f_bitan, 0), vec4(my_f_norm, 0), vec4(0, 0, 0, 1));
            mat4 invfoo = foo * inverse(foo * all_mat);
            vec4 my_f_pos = invfoo * (old_coord);//vec4(f_pos, 1.0);
            vec4 my_f_proj = all_mat * my_f_pos;
            if (my_f_proj.z <= depth) {
                new_f_pos = my_f_pos;
                f_norm = my_f_norm;
                depth = my_f_proj.z;
            }
	    }
        // f_pos = new_f_pos.xyz;
    } */

    // Test for distance to all 6 sides of the enclosing cube.
    // if (/*any(lessThan(fract(f_pos.xy), 0.01))*/fract_pos.x <= 0.1) {
    //     f_norm = faceforward(vec3(-1, 0, 0), f_norm, vec3(1, 0, 0));
    //     f_tan = vec3(0, 1, 0);
    // } else if (fract_pos.y <= 0.1) {
    //     f_norm = faceforward(vec3(0, -1, 0), f_norm, vec3(0, 1, 0));
    //     f_tan = vec3(0, 0, 1);
    // } else {
    //     f_norm = faceforward(vec3(0, 0, -1), f_norm, vec3(0, 0, 1));
    //     f_tan = vec3(1, 0, 0);
    // }
    // vec3 f_bitan = cross(f_norm, f_tan);

    // mat4 foo = mat4(vec4(f_tan, 0), vec4(f_bitan, 0), vec4(f_norm, 0), vec4(0, 0, 0, 1));
    // mat4 invfoo = foo * inverse(foo * all_mat);
    // vec3 old_coord = all_mat * vec4(f_pos.xyz, 1.0);
    // vec4 new_f_pos = invfoo * (old_coord);//vec4(f_pos, 1.0);
	vec3 f_col = lod_col(f_pos.xy);
    // vec3 f_norm = faceforward(f_norm, cam_pos.xyz - f_pos, -f_norm);
    // vec3 f_up = faceforward(cam_pos.xyz - f_pos, vec3(0.0, 0.0, -1.0), cam_pos.xyz - f_pos);
    // vec3 f_norm = faceforward(f_norm, /*vec3(cam_pos.xyz - f_pos.xyz)*/vec3(0.0, 0.0, -1.0), f_norm);

    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    vec3 view_dir = -cam_to_frag;
    // vec3 view_dir = normalize(f_pos - cam_pos.xyz);

    // Note: because voxels, we reduce the normal for reflections to just its z component, dpendng on distance to camera.
    // Idea: the closer we are to facing top-down, the more the norm should tend towards up-z.
    // vec3 l_norm; // = vec3(0.0, 0.0, 1.0);
    // vec3 l_norm = normalize(vec3(f_norm.x / max(abs(f_norm.x), 0.001), f_norm.y / max(abs(f_norm.y), 0.001), f_norm.z / max(abs(f_norm.z), 0.001)));
    // vec3 l_factor = 1.0 / (1.0 + max(abs(/*f_pos - cam_pos.xyz*//*-vec3(vert_pos4) / vert_pos4.w*/vec3(f_pos.xy, 0.0) - vec3(/*cam_pos*/focus_pos.xy, cam_to_frag)) - vec3(view_distance.x, view_distance.x, 0.0), 0.0) / vec3(32.0 * 2.0, 32.0 * 2.0, 1.0));
    // l_factor.z =
    // vec4 focus_pos4 = view_mat * vec4(focus_pos.xyz, 1.0);
    // vec3 focus_dir = normalize(-vec3(focus_pos4) / focus_pos4.w);

    // float l_factor = 1.0 - pow(clamp(0.5 + 0.5 * dot(/*-view_dir*/-cam_to_frag, l_norm), 0.0, 1.0), 2.0);//1.0 / (1.0 + 0.5 * pow(max(distance(/*focus_pos.xy*/vec3(focus_pos.xy, /*vert_pos4.z / vert_pos4.w*/f_pos.z), vec3(f_pos.xy, f_pos.z))/* - view_distance.x*/ - 32.0, 0.0) / (32.0 * 1.0), /*0.5*/1.0));
    // l_factor = 1.0;
    // l_norm = normalize(mix(l_norm, f_norm, l_factor));
    // l_norm = f_norm;

    /* l_norm = normalize(vec3(
            mix(l_norm.x, f_norm.x, clamp(pow(f_norm.x * 0.5, 64), 0, 1)),
            mix(-1.0, 1.0, clamp(pow(f_norm.y * 0.5, 64), 0, 1)),
            mix(-1.0, 1.0, clamp(pow(f_norm.z * 0.5, 64), 0, 1))
        )); */
    // f_norm = mix(l_norm, f_norm, min(1.0 / max(cam_to_frag, 0.001), 1.0));
    /* vec3 l_norm = normalize(vec3(
            mix(-1.0, 1.0, clamp(pow(f_norm.x * 0.5, 64), 0, 1)),
            mix(-1.0, 1.0, clamp(pow(f_norm.y * 0.5, 64), 0, 1)),
            mix(-1.0, 1.0, clamp(pow(f_norm.z * 0.5, 64), 0, 1))
        )); */

    vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x);
    // float sun_light = get_sun_brightness(sun_dir);
	// float moon_light = get_moon_brightness(moon_dir);
    // float my_alt = f_pos.z;//alt_at_real(f_pos.xy);
    // vec3 f_norm = my_norm;
    vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
    // float my_alt = alt_at(f_pos.xy);
    float shadow_alt = /*f_pos.z;*/alt_at(f_pos.xy);
    float sun_shade_frac = horizon_at2(f_shadow, shadow_alt, f_pos, sun_dir);
    float moon_shade_frac = horizon_at2(f_shadow, shadow_alt, f_pos, moon_dir);
    // float sun_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, sun_dir);
    // float moon_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, moon_dir);
    // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-f_norm, sun_dir)) * 10000.0), 0.0, 0.5);
    // NOTE: current assumption is that moon and sun shouldn't be out at the sae time.
    // This assumption is (or can at least easily be) wrong, but if we pretend it's true we avoids having to explicitly pass in a separate shadow
    // for the sun and moon (since they have different brightnesses / colors so the shadows shouldn't attenuate equally).
    float shade_frac = sun_shade_frac + moon_shade_frac;
    // float brightness_denominator = (ambient_sides + vec3(SUN_AMBIANCE * sun_light + moon_light);

    float alpha = 1.0;//0.1;//0.2;///1.0;//sqrt(2.0);
    const float n2 = 1.01;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float R_s = (f_pos.z < my_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

	vec3 emitted_light, reflected_light;
    // Use f_norm here for better shadows.
    // vec3 light_frac = light_reflection_factor(f_norm/*l_norm*/, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(/*1.0*/R_s), alpha);

	// vec3 light, diffuse_light, ambient_light;
    // get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, (0.25 * shade_frac + 0.25 * light_frac) * f_col, 0.5 * shade_frac * f_col, 0.5 * shade_frac * /*vec3(1.0)*/f_col, 2.0, emitted_light, reflected_light);
    float max_light = 0.0;
    max_light += get_sun_diffuse2(f_norm/*l_norm*/, sun_dir, moon_dir, view_dir, vec3(1.0)/* * (0.5 * light_frac + vec3(0.5 * shade_frac))*/, vec3(1.0), /*0.5 * shade_frac * *//*vec3(1.0)*//*f_col*/vec3(R_s), alpha, emitted_light, reflected_light);
    // emitted_light = vec3(1.0);
    emitted_light *= max(shade_frac, MIN_SHADOW);
    reflected_light *= shade_frac;
    max_light *= shade_frac;
    // reflected_light = vec3(0.0);

    // emitted_light += 0.5 * vec3(SUN_AMBIANCE * sun_shade_frac * sun_light + moon_shade_frac * moon_light) * f_col * (ambient_sides + 1.0);

    // Ambient lighting attempt: vertical light.
    // reflected_light += /*0.0125*/0.15 * 0.25 * _col * light_reflection_factor(f_norm, cam_to_frag, vec3(0, 0, -1.0), 0.5 * f_col, 0.5 * f_col, 2.0);
    // emitted_light += /*0.0125*/0.25 * f_col * ;
	// vec3 light, diffuse_light, ambient_light;
	// get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	// vec3 surf_color = illuminate(f_col, light, diffuse_light, ambient_light);
	// f_col = f_col + (hash(vec4(floor(vec3(focus_pos.xy + splay(v_pos_orig), f_pos.z)) * 3.0 - round(f_norm) * 0.5, 0)) - 0.5) * 0.05; // Small-scale noise
    vec3 surf_color = /*illuminate(emitted_light, reflected_light)*/illuminate(max_light, f_col * emitted_light, f_col * reflected_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);

	vec4 clouds;
	vec3 fog_color = get_sky_color(cam_to_frag/*view_dir*/, time_of_day.x, cam_pos.xyz, f_pos, 1.0, true, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);
    // vec3 color = surf_color;

	// float mist_factor = max(1 - (f_pos.z + (texture(t_noise, f_pos.xy * 0.0005 + time_of_day.x * 0.0003).x - 0.5) * 128.0) / 400.0, 0.0);
	// //float mist_factor = f_norm.z * 2.0;
	// color = mix(color, vec3(1.0) * /*diffuse_light*/reflected_light, clamp(mist_factor * 0.00005 * distance(f_pos.xy, focus_pos.xy), 0, 0.3));
    // color = surf_color;

	tgt_color = vec4(color, 1.0);
}
