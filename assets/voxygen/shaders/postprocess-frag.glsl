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

#include <globals.glsl>
// Note: The sampler uniform is declared here because it differs for MSAA
#include <anti-aliasing.glsl>
#include <srgb.glsl>

in vec2 f_pos;

layout (std140)
uniform u_locals {
	vec4 nul;
};

out vec4 tgt_color;

vec3 rgb2hsv(vec3 c) {
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

vec3 illuminate(float max_light, vec3 view_dir, /*vec3 max_light, */vec3 emitted, vec3 reflected) {
    const float NIGHT_EXPOSURE = 10.0;
    const float DUSK_EXPOSURE = 2.0;//0.8;
    const float DAY_EXPOSURE = 1.0;//0.7;

    const float DAY_SATURATION = 1.0;
    const float DUSK_SATURATION = 0.6;
    const float NIGHT_SATURATION = 0.1;

    const float gamma = /*0.5*//*1.*0*/1.0;//1.0;
    /* float light = length(emitted + reflected);
    float color = srgb_to_linear(emitted + reflected);
    float avg_col = (color.r + color.g + color.b) / 3.0;
    return ((color - avg_col) * light + reflected * avg_col) * (emitted + reflected); */
    // float max_intensity = vec3(1.0);
    vec3 color = emitted + reflected;
    float lum = rel_luminance(color);
    // float lum_sky = lum - max_light;

    // vec3 sun_dir = get_sun_dir(time_of_day.x);
    // vec3 moon_dir = get_moon_dir(time_of_day.x);
    // float sky_light = rel_luminance(
    //         get_sun_color(sun_dir) * get_sun_brightness(sun_dir) * SUN_COLOR_FACTOR +
    //         get_moon_color(moon_dir) * get_moon_brightness(moon_dir));
    float sky_light = lum;

    // Tone mapped value.
    // vec3 T = /*color*//*lum*/color;//normalize(color) * lum / (1.0 + lum);
    // float alpha = 0.5;//2.0;
    // float alpha = mix(
	// 	mix(
	// 		DUSK_EXPOSURE,
	// 		NIGHT_EXPOSURE,
	// 		max(sun_dir.z, 0)
	// 	),
	// 	DAY_EXPOSURE,
	// 	max(-sun_dir.z, 0)
	// );
    float alpha = 1.0;//log(1.0 - lum) / lum;
    // vec3 now_light = moon_dir.z < 0 ? moon_dir : sun_dir;
    // float cos_view_light = dot(-now_light, view_dir);
    // alpha *= exp(1.0 - cos_view_light);
    // sky_light *= 1.0 - log(1.0 + view_dir.z);
    float alph = sky_light > 0.0 && max_light > 0.0 ? mix(1.0 / log(/*1.0*//*1.0 + *//*lum_sky + */1.0 + max_light / (0.0 + sky_light)), 1.0, clamp(max_light - sky_light, 0.0, 1.0)) : 1.0;
    alpha = alpha * alph;// min(alph, 1.0);//((max_light > 0.0 && max_light > sky_light /* && sky_light > 0.0*/) ? /*1.0*/1.0 / log(/*1.0*//*1.0 + *//*lum_sky + */1.0 + max_light - (0.0 + sky_light)) : 1.0);
    // alpha = alpha * min(1.0, (max_light == 0.0 ? 1.0 : (1.0 + abs(lum_sky)) / /*(1.0 + max_light)*/max_light));

    vec3 col_adjusted = lum == 0.0 ? vec3(0.0) : color / lum;

    // float L = lum == 0.0 ? 0.0 : log(lum);


    // // float B = T;
    // // float B = L + log(alpha);
    // float B = lum;

    // float D = L - B;

    // float o = 0.0;//log(PERSISTENT_AMBIANCE);
    // float scale = /*-alpha*/-alpha;//1.0;

    // float B_ = (B - o) * scale;

    // // float T = lum;
    // float O = exp(B_ + D);

    float T = 1.0 - exp(-alpha * lum);//lum / (1.0 + lum);
    // float T = lum;

    // Heuristic desaturation
    // const float s = 0.8;
    float s = 1.0;
    // float s = mix(
	// 	mix(
	// 		DUSK_SATURATION,
	// 		NIGHT_SATURATION,
	// 		max(sun_dir.z, 0)
	// 	),
	// 	DAY_SATURATION,
	// 	max(-sun_dir.z, 0)
	// );
    // s = max(s, (max_light) / (1.0 + s));
    // s = max(s, max_light / (1.0 + max_light));
    // s = max_light / (1.0 + max_light);

    vec3 c = pow(col_adjusted, vec3(s)) * T;
    // vec3 c = col_adjusted * T;
    // vec3 c = sqrt(col_adjusted) * T;
    // vec3 c = /*col_adjusted * */col_adjusted * T;

    return c;
    // float sum_col = color.r + color.g + color.b;
    // return /*srgb_to_linear*/(/*0.5*//*0.125 * */vec3(pow(color.x, gamma), pow(color.y, gamma), pow(color.z, gamma)));
}

void main() {
	vec2 uv = (f_pos + 1.0) * 0.5;

	/* if (medium.x == 1u) {
		uv = clamp(uv + vec2(sin(uv.y * 16.0 + tick.x), sin(uv.x * 24.0 + tick.x)) * 0.005, 0, 1);
	} */

    vec2 c_uv = vec2(0.5);//uv;//vec2(0.5);//uv;
    vec2 delta = /*sqrt*//*sqrt(2.0) / 2.0*//*sqrt(2.0) / 2.0*//*0.5 - */min(uv, 1.0 - uv);//min(uv * (1.0 - uv), 0.25) * 2.0;
    // delta = /*sqrt(2.0) / 2.0 - */sqrt(vec2(dot(delta, delta)));
    // delta = 0.5 - vec2(min(delta.x, delta.y));
    delta = vec2(0.25);//vec2(dot(/*0.5 - */delta, /*0.5 - */delta));//vec2(min(delta.x, delta.y));//sqrt(2.0) * (0.5 - vec2(min(delta.x, delta.y)));
    // delta = vec2(sqrt(dot(delta, delta)));
    // vec2 delta = /*sqrt*//*sqrt(2.0) / 2.0*//*sqrt(2.0) / 2.0*/1.0 - vec2(sqrt(dot(uv, 1.0 - uv)));//min(uv * (1.0 - uv), 0.25) * 2.0;
    // float delta = /*sqrt*//*sqrt(2.0) / 2.0*//*sqrt(2.0) / 2.0*/1.0 - (dot(uv - 0.5, uv - 0.5));//0.01;//25;
    // vec2 delta = /*sqrt*//*sqrt(2.0) / 2.0*//*sqrt(2.0) / 2.0*/sqrt(uv * (1.0 - uv));//min(uv * (1.0 - uv), 0.25) * 2.0;

    // float bright_color0 = rel_luminance(texelFetch/*texture*/(src_color, ivec2(clamp(c_uv + vec2(0.0, 0.0), 0.0, 1.0) * screen_res.xy/* / 50*/)/* * 50*/, 0).rgb);
    // float bright_color1 = rel_luminance(texelFetch/*texture*/(src_color, ivec2(clamp(c_uv + vec2(delta.x, delta.y), 0.0, 1.0) * screen_res.xy/* / 50*/)/* * 50*/, 0).rgb);
    // float bright_color2 = rel_luminance(texelFetch/*texture*/(src_color, ivec2(clamp(c_uv + vec2(delta.x, -delta.y), 0.0, 1.0) * screen_res.xy/* / 50*/)/* * 50*/, 0).rgb);
    // float bright_color3 = rel_luminance(texelFetch/*texture*/(src_color, ivec2(clamp(c_uv + vec2(-delta.x, delta.y), 0.0, 1.0) * screen_res.xy/* / 50*/)/* * 50*/, 0).rgb);
    // float bright_color4 = rel_luminance(texelFetch/*texture*/(src_color, ivec2(clamp(c_uv + vec2(-delta.x, -delta.y), 0.0, 1.0) * screen_res.xy/* / 50*/)/* * 50*/, 0).rgb);

    // float bright_color0 = rel_luminance(texture(src_color, /*ivec2*/(clamp(c_uv + vec2(0.0, 0.0), 0.0, 1.0)/* * screen_res.xy*//* / 50*/)/* * 50*/, 0).rgb);
    // float bright_color1 = rel_luminance(texture(src_color, /*ivec2*/(clamp(c_uv + vec2(delta, delta), 0.0, 1.0)/* * screen_res.xy*//* / 50*/)/* * 50*/, 0).rgb);
    // float bright_color2 = rel_luminance(texture(src_color, /*ivec2*/(clamp(c_uv + vec2(delta, -delta), 0.0, 1.0)/* * screen_res.xy*//* / 50*/)/* * 50*/, 0).rgb);
    // float bright_color3 = rel_luminance(texture(src_color, /*ivec2*/(clamp(c_uv + vec2(-delta, delta), 0.0, 1.0)/* * screen_res.xy*//* / 50*/)/* * 50*/, 0).rgb);
    // float bright_color4 = rel_luminance(texture(src_color, /*ivec2*/(clamp(c_uv + vec2(-delta, -delta), 0.0, 1.0)/* * screen_res.xy*//* / 50*/)/* * 50*/, 0).rgb);

    // float bright_color = max(bright_color0, max(bright_color1, max(bright_color2, max(bright_color3, bright_color4))));// / 2.0;// / 5.0;

    // float bright_color = (bright_color0 + bright_color1 + bright_color2 + bright_color3 + bright_color4) / 5.0;

	vec4 aa_color = aa_apply(src_color, uv * screen_res.xy, screen_res.xy);

    // aa_color.rgb = illuminate(1.0 - 1.0 / (1.0 + bright_color), normalize(cam_pos.xyz - focus_pos.xyz), /*vec3 max_light, */vec3(0.0), aa_color.rgb);

	//vec4 hsva_color = vec4(rgb2hsv(fxaa_color.rgb), fxaa_color.a);
	//hsva_color.y *= 1.45;
	//hsva_color.z *= 0.85;
	//hsva_color.z = 1.0 - 1.0 / (1.0 * hsva_color.z + 1.0);
	//vec4 final_color = vec4(hsv2rgb(hsva_color.rgb), hsva_color.a);

	vec4 final_color = pow(aa_color, gamma);

#if (FLUID_MODE == FLUID_MODE_CHEAP)
	if (medium.x == 1u) {
		final_color *= vec4(0.2, 0.2, 0.8, 1.0);
	}
#endif

	tgt_color = vec4(final_color.rgb, 1);
}
