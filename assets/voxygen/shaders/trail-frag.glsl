#version 420 core

layout(location = 0) in vec3 f_pos;

layout(location = 0) out vec4 tgt_color;

void main() {
    tgt_color = vec4(.55, .92, 1.0, .02);
}
