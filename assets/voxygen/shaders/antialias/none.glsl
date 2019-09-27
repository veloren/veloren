uniform sampler2D src_color;

vec4 aa_apply(sampler2D tex, vec2 fragCoord, vec2 resolution) {
	return texture(src_color, fragCoord / resolution);
}