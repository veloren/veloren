#version 330 core

in vec3 vert_pos;
in vec3 vert_norm;
in vec4 vert_col;

uniform constants {
	mat4 model_mat;
    mat4 view_mat;
	mat4 perspective_mat;
};

out vec3 frag_pos;
out vec3 frag_norm;
out vec4 frag_col;

void main() {
	frag_pos = vert_pos;
	frag_norm = vert_norm;
    frag_col = vert_col;
    gl_Position = perspective_mat * view_mat * model_mat * vec4(vert_pos, 1);
}
