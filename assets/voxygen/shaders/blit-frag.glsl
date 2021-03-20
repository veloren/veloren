#version 420 core

layout(set = 0, binding = 0)
uniform texture2D t_src_color;
layout(set = 0, binding = 1)
uniform sampler s_src_color;

layout(location = 0) in vec2 uv;

layout(location = 0) out vec4 tgt_color;

void main() {
    vec4 color = texture(sampler2D(t_src_color, s_src_color), uv);

    tgt_color = vec4(color.rgb, 1);
}
