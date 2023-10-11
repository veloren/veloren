#ifndef GLOBALS_GLSL
#define GLOBALS_GLSL

layout(std140, set = 0, binding = 0) uniform u_globals {
    mat4 view_mat;
    mat4 proj_mat;
    mat4 all_mat;
    vec4 cam_pos;
    vec4 focus_off;
    vec4 focus_pos;
    vec4 view_distance;
    // .x = time of day, repeats every day.
    // .y = a continuous value for what day it is. Repeats every `tick_overflow` for precisions sake.
    vec4 time_of_day;
    vec4 sun_dir;
    vec4 moon_dir;
    // .x = The `Time` resource, repeated every `tick_overflow`
    // .y = a floored (`Time` / `tick_overflow`)
    // .z = `Time`, not recommended to be used as it might have low precision
    vec4 tick;
    vec4 screen_res;
    uvec4 light_shadow_count;
    vec4 shadow_proj_factors;
    uvec4 medium;
    ivec4 select_pos;
    vec4 gamma_exposure;
    vec4 last_lightning;
    vec2 wind_vel;
    float ambiance;
    // 0 - FirstPerson
    // 1 - ThirdPerson
    uint cam_mode;
    float sprite_render_distance;
    float globals_dummy; // Fix alignment.
};

// Specifies the pattern used in the player dithering
mat4 threshold_matrix = mat4(
    vec4(1.0 / 17.0,  9.0 / 17.0,  3.0 / 17.0, 11.0 / 17.0),
    vec4(13.0 / 17.0,  5.0 / 17.0, 15.0 / 17.0,  7.0 / 17.0),
    vec4(4.0 / 17.0, 12.0 / 17.0,  2.0 / 17.0, 10.0 / 17.0),
    vec4(16.0 / 17.0,  8.0 / 17.0, 14.0 / 17.0,  6.0 / 17.0)
);
float distance_divider = 2;
float shadow_dithering = 0.5;

float tick_overflow = 300000.0;

// Get a scaled time with an offset that loops at a period.
float tick_loop(float period, float scale, float offset) {
    float loop = tick_overflow * scale;
    float rem = mod(loop, period);
    float rest = rem * tick.y;

    return mod(rest + tick.x * scale + offset, period);
}

float tick_loop(float period) {
    return tick_loop(period, 1.0, 0.0);
}


vec4 tick_loop4(float period, vec4 scale, vec4 offset) {
    vec4 loop = tick_overflow * scale;
    vec4 rem = mod(loop, period);
    vec4 rest = rem * tick.y;

    return mod(rest + tick.x * scale + offset, period);
}

// Only works if t happened within tick_overflow
float time_since(float t) {
    return tick.x < t ? (tick_overflow - t + tick.x) : (tick.x - t); 
}

#endif
