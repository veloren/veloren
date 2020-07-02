#version 330 core

#include <globals.glsl>

in vec2 v_pos;
in vec2 v_uv;
in vec2 v_center;
in vec4 v_color;
in uint v_mode;

layout (std140)
uniform u_locals {
	vec4 w_pos;
};

uniform sampler2D u_tex;

out vec2 f_uv;
flat out uint f_mode;
out vec4 f_color;

void main() {
    f_color = v_color;

    // vec2 v_pos = vec2(-1.0,1.0) * v_pos;
    /* f_uv = vec2(1.0,1.0) * v_uv; */
    // vec2 v_uv = vec2(1.0,-1.0) * v_uv;

    if (w_pos.w == 1.0) {
        f_uv = v_uv;
        // Fixed scale In-game element
        vec4 projected_pos = /*proj_mat * view_mat*/all_mat * vec4(w_pos.xyz - focus_off.xyz, 1.0);
        gl_Position = vec4(projected_pos.xy / projected_pos.w + v_pos/* * projected_pos.w*/, -1.0, /*projected_pos.w*/1.0);
    } else if (v_mode == uint(3)) {
        // HACK: North facing source rectangle.
        vec2 look_at_dir = normalize(vec2(-view_mat[0][2], -view_mat[1][2]));
        mat2 look_at = mat2(look_at_dir.y, look_at_dir.x, -look_at_dir.x, look_at_dir.y);
        f_uv = v_center + look_at * (v_uv - v_center);
        gl_Position = vec4(v_pos, -1.0, 1.0);
    } else if (v_mode == uint(5)) {
        // HACK: North facing target rectangle.
        f_uv = v_uv;
        float aspect_ratio = screen_res.x / screen_res.y;
        vec2 look_at_dir = normalize(vec2(-view_mat[0][2], -view_mat[1][2]));
        mat2 look_at = mat2(look_at_dir.y, -look_at_dir.x, look_at_dir.x, look_at_dir.y);
        vec2 v_len = v_pos - v_center;
        vec2 v_proj = look_at * vec2(v_len.x, v_len.y / aspect_ratio);
        gl_Position = vec4(v_center + vec2(v_proj.x, v_proj.y * aspect_ratio), -1.0, 1.0);
    } else {
        // Interface element
        f_uv = v_uv;
        gl_Position = vec4(v_pos, -1.0, 1.0);
    }
    f_mode = v_mode;
}
