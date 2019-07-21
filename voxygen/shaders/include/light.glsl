float attenuation_strength(vec3 rpos) {
	return 1.0 / (rpos.x * rpos.x + rpos.y * rpos.y + rpos.z * rpos.z);
}

vec3 light_at(vec3 wpos, vec3 wnorm) {
	const float LIGHT_AMBIENCE = 0.1;

	vec3 light = vec3(0);
	for (uint i = 0u; i < light_count.x; i ++) {
		vec3 light_pos = lights[i].light_pos.xyz;
		float strength = attenuation_strength(wpos - light_pos);

		if (strength < 0.001) {
			continue;
		}

		light += strength
			* lights[i].light_col.rgb
			* clamp(dot(normalize(light_pos - wpos), wnorm), LIGHT_AMBIENCE, 1.0);
	}
	return light;
}
