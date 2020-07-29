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
        gl_Position = vec4(v_pos, -1.0, 1.0);
        vec2 look_at_dir = normalize(vec2(-view_mat[0][2], -view_mat[1][2]));
        // TODO: Consider cleaning up matrix to something more efficient (e.g. a mat3).
        vec2 aspect_ratio = textureSize(u_tex, 0).yx;
        mat2 look_at = mat2(look_at_dir.y, look_at_dir.x, -look_at_dir.x, look_at_dir.y);
        vec2 v_centered = (v_uv - v_center) / aspect_ratio;
        vec2 v_rotated = look_at * v_centered;
        f_uv = aspect_ratio * v_rotated + v_center;
    } else if (v_mode == uint(5)) {
        // HACK: North facing target rectangle.
        f_uv = v_uv;
        vec2 look_at_dir = normalize(vec2(-view_mat[0][2], -view_mat[1][2]));
        // TODO: Consider cleaning up matrix to something more efficient (e.g. a mat3).
        vec2 aspect_ratio = screen_res.yx;
        mat2 look_at = mat2(look_at_dir.y, -look_at_dir.x, look_at_dir.x, look_at_dir.y);
        vec2 v_centered = (v_pos - v_center) / aspect_ratio;
        vec2 v_rotated = look_at * v_centered;
        gl_Position = vec4(aspect_ratio * v_rotated + v_center, -1.0, 1.0);
    } else {
        // Interface element
        f_uv = v_uv;
        gl_Position = vec4(v_pos, -1.0, 1.0);
    }
    f_mode = v_mode;
}
