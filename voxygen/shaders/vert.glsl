#version 330 core

in vec3 vert_pos;
in vec3 vert_norm;
in vec3 vert_col;

uniform constants {
    mat4 uni_trans;
};

out vec3 frag_col;

void main() {
    frag_col = vert_col;
    gl_Position = vec4(vert_pos, 1) * uni_trans;
}
