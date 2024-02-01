#version 440 core
#extension GL_EXT_samplerless_texture_functions : enable

layout(set = 0, binding = 0)
uniform texture2D source_texture;

layout(location = 0) in vec2 source_coords;

layout(location = 0) out vec4 target_color;

void main() {
    // We get free nonlinear -> linear conversion when sampling from srgb texture;
    vec4 linear = texelFetch(source_texture, ivec2(source_coords), 0);
    vec4 premultiplied_linear = vec4(linear.rgb * linear.a, linear.a);
    // We get free linear -> nonlinear conversion rendering to srgb texture.
    target_color = premultiplied_linear;
}
