#version 420 core

#include <globals.glsl>

layout(location = 0) in vec2 v_pos;
layout(location = 1) in vec2 v_uv;
layout(location = 2) in vec4 v_color;
layout(location = 3) in vec2 v_center;
layout(location = 4) in uint v_mode;

layout (std140, set = 1, binding = 0)
uniform u_locals {
    vec4 w_pos;
};

layout(set = 2, binding = 0)
uniform texture2D t_tex;
layout(set = 2, binding = 1)
uniform sampler s_tex;

layout(location = 0) out vec2 f_uv;
layout(location = 1) out vec4 f_color;
layout(location = 2) flat out uint f_mode;

void main() {
    f_color = v_color;

    // vec2 v_pos = vec2(-1.0,1.0) * v_pos;
    /* f_uv = vec2(1.0,1.0) * v_uv; */
    // vec2 v_uv = vec2(1.0,-1.0) * v_uv;

    if (w_pos.w == 1.0) {
        f_uv = v_uv;
        // Fixed scale In-game element
        vec4 projected_pos = /*proj_mat * view_mat*/all_mat * vec4(w_pos.xyz - focus_off.xyz, 1.0);
        gl_Position = vec4(projected_pos.xy / projected_pos.w + v_pos/* * projected_pos.w*/, 0.5, /*projected_pos.w*/1.0);
    } else if (v_mode == uint(3)) {
        // HACK: North facing source rectangle.
        gl_Position = vec4(v_pos, 0.5, 1.0);
        vec2 look_at_dir = normalize(vec2(-view_mat[0][2], -view_mat[1][2]));
        // TODO: Consider cleaning up matrix to something more efficient (e.g. a mat3).
        vec2 aspect_ratio = textureSize(sampler2D(t_tex, s_tex), 0).yx;
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
        gl_Position = vec4(aspect_ratio * v_rotated + v_center, 0.5, 1.0);
    } else {
        // Interface element
        f_uv = v_uv;
        gl_Position = vec4(v_pos, 0.5, 1.0);
    }

    f_mode = v_mode;
}
