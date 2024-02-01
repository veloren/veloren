#version 440 core

#include <globals.glsl>

layout (location = 0)
in vec3 v_pos;
layout (location = 1)
in vec4 v_color;
layout (location = 2)
in vec3 v_norm;

layout (std140, set = 2, binding = 0)
uniform u_locals {
    vec4 w_pos;
    vec4 w_color;
    vec4 w_ori;
};

layout (location = 0)
out vec4 f_color;
layout (location = 1)
out vec3 f_pos;
layout (location = 2)
out vec3 f_norm;

void main() {
    f_color = w_color * v_color;

    // Build rotation matrix
    // https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles#Rotation_matrices
    mat3 rotation_matrix;
    float q0 = w_ori[3];
    float q1 = w_ori[0];
    float q2 = w_ori[1];
    float q3 = w_ori[2];

    float r00 = 1 - 2 * (pow(q2, 2) + pow(q3, 2));
    float r01 = 2 * (q1 * q2 - q0 * q3);
    float r02 = 2 * (q0 * q2 + q1 * q3);
    rotation_matrix[0] = vec3(r00, r01, r02);

    float r10 = 2 * (q1 * q2 + q0 * q3);
    float r11 = 1 - 2 * (pow(q1, 2) + pow(q3, 2));
    float r12 = 2 * (q2 * q3 - q0 * q1);
    rotation_matrix[1] = vec3(r10, r11, r12);

    float r20 = 2 * (q1 * q3 - q0 * q2);
    float r21 = 2 * (q0 * q1 + q2 * q3);
    float r22 = 1 - 2 * (pow(q1, 2) + pow(q2, 2));
    rotation_matrix[2] = vec3(r20, r21, r22);

    f_pos = (v_pos * rotation_matrix + w_pos.xyz) - focus_off.xyz;
    f_norm = normalize(v_norm);
    gl_Position = all_mat * vec4(f_pos, 1);
}
