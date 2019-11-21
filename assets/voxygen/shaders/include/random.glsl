uniform sampler2D t_noise;

float hash(vec4 p) {
	p = fract( p*0.3183099+.1);
	p *= 17.0;
    return (fract(p.x*p.y*p.z*p.w*(p.x+p.y+p.z+p.w)) - 0.5) * 2.0;
}

float snoise(in vec4 x) {
    vec4 p = floor(x);
    vec4 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
		mix(
			mix(
				mix(hash(p + vec4(0,0,0,0)), hash(p + vec4(1,0,0,0)), f.x),
				mix(hash(p + vec4(0,1,0,0)), hash(p + vec4(1,1,0,0)), f.x),
				f.y),
			mix(
				mix(hash(p + vec4(0,0,1,0)), hash(p + vec4(1,0,1,0)), f.x),
				mix(hash(p + vec4(0,1,1,0)), hash(p + vec4(1,1,1,0)), f.x),
				f.y),
			f.z),
		mix(
			mix(
				mix(hash(p + vec4(0,0,0,1)), hash(p + vec4(1,0,0,1)), f.x),
				mix(hash(p + vec4(0,1,0,1)), hash(p + vec4(1,1,0,1)), f.x),
				f.y),
			mix(
				mix(hash(p + vec4(0,0,1,1)), hash(p + vec4(1,0,1,1)), f.x),
				mix(hash(p + vec4(0,1,1,1)), hash(p + vec4(1,1,1,1)), f.x),
				f.y),
			f.z),
		f.w);
}

vec3 rand_perm_3(vec3 pos) {
	return sin(pos * vec3(1473.7 * pos.z + 472.3, 8891.1 * pos.x + 723.1, 3813.3 * pos.y + 982.5));
}

vec4 rand_perm_4(vec4 pos) {
	return sin(473.3 * pos * vec4(317.3 * pos.w + 917.7, 1473.7 * pos.z + 472.3, 8891.1 * pos.x + 723.1, 3813.3 * pos.y + 982.5) / pos.yxwz);
}

vec3 smooth_rand(vec3 pos, float lerp_axis) {
	return vec3(snoise(vec4(pos, lerp_axis)), snoise(vec4(pos + 400.0, lerp_axis)), snoise(vec4(pos + 1000.0, lerp_axis)));
	vec3 r0 = rand_perm_3(vec3(pos.x, pos.y, pos.z) + floor(lerp_axis));
	vec3 r1 = rand_perm_3(vec3(pos.x, pos.y, pos.z) + floor(lerp_axis + 1.0));
	return r0 + (r1 - r0) * fract(lerp_axis);
}
