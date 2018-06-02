#version 330 core

in vec3 frag_col;
out vec4 target;

void main() {
    target = vec4(frag_col, 1);
}
