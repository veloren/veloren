#version 330 core

#include <globals.glsl>
// Note: The sampler uniform is declared here because it differs for MSAA
#include <anti-aliasing.glsl>

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

void main() {
	vec2 uv = (f_pos + 1.0) * 0.5;

	if (medium.x == 1u) {
		uv = clamp(uv + vec2(sin(uv.y * 16.0 + tick.x), sin(uv.x * 24.0 + tick.x)) * 0.005, 0, 1);
	}


	vec4 aa_color = aa_apply(src_color, uv * screen_res.xy, screen_res.xy);

	//vec4 hsva_color = vec4(rgb2hsv(fxaa_color.rgb), fxaa_color.a);
	//hsva_color.y *= 1.45;
	//hsva_color.z *= 0.85;
	//hsva_color.z = 1.0 - 1.0 / (1.0 * hsva_color.z + 1.0);
	//vec4 final_color = vec4(hsv2rgb(hsva_color.rgb), hsva_color.a);

    vec4 gamma = vec4(1.0);
	
	vec4 final_color = pow(aa_color, gamma);

	if (medium.x == 1u) {
		final_color *= vec4(0.2, 0.2, 0.8, 1.0);
	}

	tgt_color = vec4(final_color.rgb, 1);
}
