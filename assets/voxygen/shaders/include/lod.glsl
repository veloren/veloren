#include <random.glsl>

float alt_at(vec2 pos) {
	return 0.0
		+ pow(texture(t_noise, pos * 0.00005).x * 1.4, 3.0) * 1000.0
		+ texture(t_noise, pos * 0.001).x * 100.0
		+ texture(t_noise, pos * 0.003).x * 30.0;
}

vec2 splay(vec2 pos, float e) {
	return pos * pow(length(pos), e);
}

vec3 lod_pos(vec2 v_pos) {
	vec2 hpos = focus_pos.xy + splay(v_pos, 3.0) * 20000.0;
	return vec3(hpos, alt_at(hpos));
}

vec3 lod_norm(vec2 pos) {
	float alt00 = alt_at(pos);
	float alt10 = alt_at(pos + vec2(100, 0));
	float alt01 = alt_at(pos + vec2(0, 100));
	float slope = abs(alt00 - alt10) + abs(alt00 - alt01);

	return normalize(vec3(
		(alt00 - alt10) / 100,
		(alt00 - alt01) / 100,
		100 / slope
	));
}

vec3 lod_col(vec2 pos) {
	vec3 warmth = mix(
		vec3(0.05, 0.4, 0.15),
		vec3(0.5, 0.4, 0.0),
		(texture(t_noise, pos * 0.0002).x - 0.5) * 2.0 + 0.5
	);

	vec3 color = mix(
		warmth,
		vec3(0.5, 0.5, 0.5),
		alt_at(pos) / 1200.0
	);

	return color;
}
