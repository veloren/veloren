#version 440 core

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

vec4 simplesample(vec2 uv) {
    return textureLod(sampler2D(t_src_color, s_src_color), uv, 0);
}

// From: https://community.arm.com/cfs-file/__key/communityserver-blogs-components-weblogfiles/00-00-00-20-66/siggraph2015_2D00_mmg_2D00_marius_2D00_notes.pdf
vec4 upsample(vec2 uv, vec2 halfpixel) {
    vec4 sum = simplesample(uv + vec2(-halfpixel.x * 2.0, 0.0));
    sum += simplesample(uv + vec2(-halfpixel.x, halfpixel.y)) * 2.0;
    sum += simplesample(uv + vec2(0.0, halfpixel.y * 2.0));
    sum += simplesample(uv + vec2(halfpixel.x, halfpixel.y)) * 2.0;
    sum += simplesample(uv + vec2(halfpixel.x * 2.0, 0.0));
    sum += simplesample(uv + vec2(halfpixel.x, -halfpixel.y)) * 2.0;
    sum += simplesample(uv + vec2(0.0, -halfpixel.y * 2.0));
    sum += simplesample(uv + vec2(-halfpixel.x, -halfpixel.y)) * 2.0;
    return sum / 12.0;
}

void main() {
    tgt_color = upsample(uv, halfpixel);
}
