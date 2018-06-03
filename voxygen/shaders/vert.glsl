#version 330 core

in vec3 vert_pos;
in vec3 vert_norm;
in vec3 vert_col;

uniform constants {
    mat4 camera_mat;
	mat4 model_mat;
};

out vec3 frag_col;

void main() {
    frag_col = vert_col;
    gl_Position = camera_mat * vec4(vert_pos, 1);
}
