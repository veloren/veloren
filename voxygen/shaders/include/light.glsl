struct Light {
	vec4 light_pos;
	vec4 light_col;
};

layout (std140)
uniform u_lights {
	Light lights[32];
};

float attenuation_strength(vec3 rpos) {
	return 1.0 / (rpos.x * rpos.x + rpos.y * rpos.y + rpos.z * rpos.z);
}

vec3 light_at(vec3 wpos, vec3 wnorm) {
	const float LIGHT_AMBIENCE = 0.025;

	vec3 light = vec3(0);
	for (uint i = 0u; i < light_count.x; i ++) {
		vec3 light_pos = lights[i].light_pos.xyz;
		float strength = attenuation_strength(wpos - light_pos);

		vec3 color = strength
			* lights[i].light_col.rgb
			* lights[i].light_col.a;

		if (max(max(color.r, color.g), color.b) < 0.002) {
			continue;
		}

		light += color * clamp(dot(normalize(light_pos - wpos), wnorm), LIGHT_AMBIENCE, 1.0);
	}
	return light;
}
