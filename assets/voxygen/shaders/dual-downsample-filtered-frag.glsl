#version 420 core

layout(set = 0, binding = 0)
uniform texture2D t_src_color;
layout(set = 0, binding = 1)
uniform sampler s_src_color;
layout(set = 0, binding = 2)

uniform u_locals {
    vec2 halfpixel;
};

layout(location = 0) in vec2 uv;

layout(location = 0) out vec4 tgt_color;

vec4 simplefetch(ivec2 uv) {
    return texelFetch(sampler2D(t_src_color, s_src_color), uv, 0);
}

// Check whether the texel color is higher than threshold, if so output as brightness color
vec4 filterDim(vec4 color) {
    // constants from: https://learnopengl.com/Advanced-Lighting/Bloom
    float brightness = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    if(brightness > 1.00)
        return vec4(color.rgb, 1.0);
    else
        return vec4(0.0, 0.0, 0.0, 1.0);
}

vec4 filteredFetch(ivec2 uv) {
    return filterDim(simplefetch(uv));
}

// Derived from: https://community.arm.com/cfs-file/__key/communityserver-blogs-components-weblogfiles/00-00-00-20-66/siggraph2015_2D00_mmg_2D00_marius_2D00_notes.pdf
vec4 filteredDownsample(vec2 uv, vec2 halfpixel) {
    vec2 tex_res = 0.5 / halfpixel;
    // coordinate of the top left texel
    //  _ _ _ _
    // |x|_|_|_|
    // |_|_|_|_|
    // |_|_|_|_|
    // |_|_|_|_|
    //
    ivec2 tl_coord = ivec2(uv * tex_res + vec2(-1.5, 1.5));
    
    // Fetch inner square
    vec4 sum = filteredFetch(tl_coord + ivec2(1, 1));
    sum += filteredFetch(tl_coord + ivec2(2, 1));
    sum += filteredFetch(tl_coord + ivec2(1, 2));
    sum += filteredFetch(tl_coord + ivec2(2, 2));
    // Weight inner square
    sum *= 5.0;
    // Fetch border
    sum += filteredFetch(tl_coord + ivec2(0, 0));
    sum += filteredFetch(tl_coord + ivec2(1, 0));
    sum += filteredFetch(tl_coord + ivec2(2, 0));
    sum += filteredFetch(tl_coord + ivec2(3, 0));
    sum += filteredFetch(tl_coord + ivec2(0, 1));
    sum += filteredFetch(tl_coord + ivec2(3, 1));
    sum += filteredFetch(tl_coord + ivec2(0, 2));
    sum += filteredFetch(tl_coord + ivec2(3, 2));
    sum += filteredFetch(tl_coord + ivec2(0, 3));
    sum += filteredFetch(tl_coord + ivec2(1, 3));
    sum += filteredFetch(tl_coord + ivec2(2, 3));
    sum += filteredFetch(tl_coord + ivec2(3, 3));
    
    return sum / 32.0;
}

vec4 naninf_filter_sample(vec2 uv) {
    vec4 color = textureLod(sampler2D(t_src_color, s_src_color), uv, 0);
    // TODO: ensure NaNs/Infs are not produced in the first place
    bvec4 nan = isnan(color);
    bvec4 inf = isinf(color);
    return mix(mix(color, vec4(0.0), nan), vec4(100.0), inf);
}

// From: https://community.arm.com/cfs-file/__key/communityserver-blogs-components-weblogfiles/00-00-00-20-66/siggraph2015_2D00_mmg_2D00_marius_2D00_notes.pdf
vec4 downsample(vec2 uv, vec2 halfpixel) {
    vec4 sum = naninf_filter_sample(uv) * 4.0;
    sum += naninf_filter_sample(uv - halfpixel.xy);
    sum += naninf_filter_sample(uv + halfpixel.xy);
    sum += naninf_filter_sample(uv + vec2(halfpixel.x, -halfpixel.y));
    sum += naninf_filter_sample(uv - vec2(halfpixel.x, -halfpixel.y));

    return sum / 8.0;
}

void main() {
    // Uncomment to experiment with filtering out dim pixels
    //tgt_color = filteredDownsample(uv, halfpixel);
    tgt_color = downsample(uv, halfpixel);
}
