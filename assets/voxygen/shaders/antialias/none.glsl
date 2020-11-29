vec4 aa_apply(texture2D tex, sampler smplr, vec2 fragCoord, vec2 resolution) {
    return texture(src_color, fragCoord / resolution);
}
