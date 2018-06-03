#version 330 core

in vec3 vert_pos;
in vec3 vert_norm;
in vec4 vert_col;

uniform constants {
    mat4 camera_mat;
	mat4 model_mat;
};

out vec4 frag_col;
out vec3 frag_norm;

void main() {
    frag_col = vert_col;
	frag_norm = vert_norm;
    gl_Position = camera_mat * vec4(vert_pos, 1);
}
