const float THRESHOLD = 0.025;
const float DEPTH_THRESHOLD = 0.05;

bool diag(
    texture2D tex, sampler smplr,
    texture2D depth_tex, sampler depth_smplr,
    const float line_thickness,
    inout vec4 sum,
    vec2 uv,
    const vec2 p1,
    const vec2 p2,
    const float aa_scale
) {
    vec4 v1 = texelFetch(sampler2D(tex, smplr), ivec2(uv + vec2(p1.x, p1.y)), 0);
    vec4 v2 = texelFetch(sampler2D(tex, smplr), ivec2(uv + vec2(p2.x, p2.y)), 0);
    float d1 = 1.0 / texelFetch(sampler2D(depth_tex, depth_smplr), ivec2(uv + vec2(p1.x, p1.y)), 0).x;
    float d2 = 1.0 / texelFetch(sampler2D(depth_tex, depth_smplr), ivec2(uv + vec2(p2.x, p2.y)), 0).x;
    if (length((v1 - v2).rb) < THRESHOLD && abs(d1 - d2) < d1 * DEPTH_THRESHOLD + 3.0) {
        vec2 dir = p2 - p1;
        vec2 lp = uv - (floor(uv + p1) + 0.5);
        dir = normalize(vec2(dir.y, -dir.x));
        float l = clamp((line_thickness - dot(lp, dir)) * aa_scale, 0.0, 1.0);
        sum = mix(sum, (v1 + v2) * 0.5, l);
        return true;
    }
    return false;
}

vec4 aa_apply(
    texture2D tex, sampler smplr,
    texture2D depth_tex, sampler depth_smplr,
    vec2 fragCoord,
    vec2 resolution
) {
    uvec2 src_sz = textureSize(sampler2D(tex, smplr), 0).xy;

    vec2 upscale = resolution / src_sz;
    vec2 ip = fragCoord / upscale;
    //start with nearest pixel as 'background'
    vec4 s = texelFetch(sampler2D(tex, smplr), ivec2(ip), 0);

    float aa_scale = upscale.x * 1.5;

    //draw anti aliased diagonal lines of surrounding pixels as 'foreground'
    if (diag(tex, smplr, depth_tex, depth_smplr, 0.4, s, ip, vec2(-1, 0), vec2(0, 1), aa_scale)) {
        diag(tex, smplr, depth_tex, depth_smplr, 0.3, s, ip, vec2(-1, 0), vec2(1, 1), aa_scale);
        diag(tex, smplr, depth_tex, depth_smplr, 0.3, s, ip, vec2(-1, -1), vec2(0, 1), aa_scale);
    }
    if (diag(tex, smplr, depth_tex, depth_smplr, 0.4, s, ip, vec2(0, 1), vec2(1, 0), aa_scale)) {
        diag(tex, smplr, depth_tex, depth_smplr, 0.3, s, ip, vec2(0, 1), vec2(1, -1), aa_scale);
        diag(tex, smplr, depth_tex, depth_smplr, 0.3, s, ip, vec2(-1, 1), vec2(1, 0), aa_scale);
    }
    if (diag(tex, smplr, depth_tex, depth_smplr, 0.4, s, ip, vec2(1, 0), vec2(0, -1), aa_scale)) {
        diag(tex, smplr, depth_tex, depth_smplr, 0.3, s, ip, vec2(1, 0), vec2(-1, -1), aa_scale);
        diag(tex, smplr, depth_tex, depth_smplr, 0.3, s, ip, vec2(1, 1), vec2(0, -1), aa_scale);
    }
    if (diag(tex, smplr, depth_tex, depth_smplr, 0.4, s, ip, vec2(0, -1), vec2(-1, 0), aa_scale)) {
        diag(tex, smplr, depth_tex, depth_smplr, 0.3, s, ip, vec2(0, -1), vec2(-1, 1), aa_scale);
        diag(tex, smplr, depth_tex, depth_smplr, 0.3, s, ip, vec2(1, -1), vec2(-1, 0), aa_scale);
    }

    return s;
}
