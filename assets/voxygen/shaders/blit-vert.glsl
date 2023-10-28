#version 440 core

layout(location = 0) out vec2 uv;

void main() {
    // Generate fullscreen triangle
    vec2 v_pos = vec2(
        float(gl_VertexIndex / 2) * 4.0 - 1.0,
        float(gl_VertexIndex % 2) * 4.0 - 1.0
    );

    uv = (v_pos * vec2(1.0, -1.0) + 1.0) * 0.5;

    gl_Position = vec4(v_pos, 0.0, 1.0);
}
