uniform sampler2DMS src_color;

vec4 aa_apply(sampler2DMS tex, vec2 fragCoord, vec2 resolution) {
	ivec2 texel_coord = ivec2(fragCoord.x, fragCoord.y);

        vec4 sample1 = texelFetch(tex, texel_coord, 0);
        vec4 sample2 = texelFetch(tex, texel_coord, 1);
        vec4 sample3 = texelFetch(tex, texel_coord, 2);
        vec4 sample4 = texelFetch(tex, texel_coord, 3);
        vec4 sample5 = texelFetch(tex, texel_coord, 4);
        vec4 sample6 = texelFetch(tex, texel_coord, 5);
        vec4 sample7 = texelFetch(tex, texel_coord, 6);
        vec4 sample8 = texelFetch(tex, texel_coord, 7);

	// Average Samples
	vec4 msaa_color = (sample1 + sample2 + sample3 + sample4 + sample5 + sample6 + sample7 + sample8) / 8.0;

	return msaa_color;
}