#version 330 core

#include <globals.glsl>

in vec3 f_pos;
flat in vec3 f_norm;
in vec3 f_col;
in float f_light;

out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>

const float RENDER_DIST = 128.0;
const float FADE_DIST = 32.0;

void main() {
	vec3 light = get_sun_diffuse(f_norm, time_of_day.x) * f_light + light_at(f_pos, f_norm);
	vec3 surf_color = f_col * light;

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x);
	vec3 color = mix(surf_color, fog_color, fog_level);

	tgt_color = vec4(color, 1.0 - clamp((distance(focus_pos.xy, f_pos.xy) - (RENDER_DIST - FADE_DIST)) / FADE_DIST, 0, 1));
}
