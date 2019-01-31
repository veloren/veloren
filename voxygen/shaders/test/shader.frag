#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(early_fragment_tests) in;

layout(location = 0) in vec4 frag_color;
layout(location = 0) out vec4 color;

void main() {
    color = frag_color;
}
