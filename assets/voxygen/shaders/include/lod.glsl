#include <random.glsl>

uniform sampler2D t_map;

vec2 pos_to_uv(vec2 pos) {
	vec2 uv_pos = (pos + 16) / 32768.0;
	return vec2(uv_pos.x, 1.0 - uv_pos.y);
}

// textureBicubic from https://stackoverflow.com/a/42179924
vec4 cubic(float v) {
    vec4 n = vec4(1.0, 2.0, 3.0, 4.0) - v;
    vec4 s = n * n * n;
    float x = s.x;
    float y = s.y - 4.0 * s.x;
    float z = s.z - 4.0 * s.y + 6.0 * s.x;
    float w = 6.0 - x - y - z;
    return vec4(x, y, z, w) * (1.0/6.0);
}

vec4 textureBicubic(sampler2D sampler, vec2 texCoords) {
   vec2 texSize = textureSize(sampler, 0);
   vec2 invTexSize = 1.0 / texSize;

   texCoords = texCoords * texSize - 0.5;


    vec2 fxy = fract(texCoords);
    texCoords -= fxy;

    vec4 xcubic = cubic(fxy.x);
    vec4 ycubic = cubic(fxy.y);

    vec4 c = texCoords.xxyy + vec2 (-0.5, +1.5).xyxy;

    vec4 s = vec4(xcubic.xz + xcubic.yw, ycubic.xz + ycubic.yw);
    vec4 offset = c + vec4 (xcubic.yw, ycubic.yw) / s;

    offset *= invTexSize.xxyy;

    vec4 sample0 = texture(sampler, offset.xz);
    vec4 sample1 = texture(sampler, offset.yz);
    vec4 sample2 = texture(sampler, offset.xw);
    vec4 sample3 = texture(sampler, offset.yw);

    float sx = s.x / (s.x + s.y);
    float sy = s.z / (s.z + s.w);

    return mix(
       mix(sample3, sample2, sx), mix(sample1, sample0, sx)
    , sy);
}

float alt_at(vec2 pos) {
	return texture(t_map, pos_to_uv(pos)).a * (1300.0) + 140.0;
		//+ (texture(t_noise, pos * 0.002).x - 0.5) * 64.0;

	return 0.0
		+ pow(texture(t_noise, pos * 0.00005).x * 1.4, 3.0) * 1000.0
		+ texture(t_noise, pos * 0.001).x * 100.0
		+ texture(t_noise, pos * 0.003).x * 30.0;
}

vec2 splay(vec2 pos) {
	return pos * pow(length(pos) * 0.5, 3.0);
}

vec3 lod_norm(vec2 pos) {
	const float SAMPLE_W = 32;

	float altx0 = alt_at(pos + vec2(-1, 0) * SAMPLE_W);
	float altx1 = alt_at(pos + vec2(1, 0) * SAMPLE_W);
	float alty0 = alt_at(pos + vec2(0, -1) * SAMPLE_W);
	float alty1 = alt_at(pos + vec2(0, 1) * SAMPLE_W);
	float slope = abs(altx1 - altx0) + abs(alty0 - alty1);

	return normalize(vec3(
		(altx0 - altx1) / SAMPLE_W,
		(alty0 - alty1) / SAMPLE_W,
		SAMPLE_W / (slope + 0.00001) // Avoid NaN
	));
}

vec3 lod_pos(vec2 v_pos, vec2 focus_pos) {
	vec2 hpos = focus_pos.xy + splay(v_pos) * 1000000.0;

	// Remove spiking by "pushing" vertices towards local optima
	vec2 nhpos = hpos;
	for (int i = 0; i < 3; i ++) {
		nhpos -= lod_norm(hpos).xy * 15.0;
	}
	hpos = hpos + normalize(nhpos - hpos + 0.001) * min(length(nhpos - hpos), 32);

	return vec3(hpos, alt_at(hpos));
}

vec3 lod_col(vec2 pos) {
	//return vec3(0, 0.5, 0);
	return textureBicubic(t_map, pos_to_uv(pos)).rgb;
		//+ (texture(t_noise, pos * 0.04 + texture(t_noise, pos * 0.005).xy * 2.0 + texture(t_noise, pos * 0.06).xy * 0.6).x - 0.5) * 0.1;
}
