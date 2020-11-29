#ifndef RANDOM_GLSL
#define RANDOM_GLSL

layout(set = 0, binding = 1) uniform texture2D t_noise;
layout(set = 0, binding = 1) uniform sampler s_noise;

float hash(vec4 p) {
    p = fract(p * 0.3183099 + 0.1) - fract(p + 23.22121);
    p *= 17.0;
    return (fract(p.x * p.y * p.z * p.w * (p.x + p.y + p.z + p.w)) - 0.5) * 2.0;
}

#define M1 1597334677U     //1719413*929
#define M2 3812015801U     //140473*2467*11
#define M3 2741598923U

float hash_one(uint q)
{
    uint n = ((M3 * q) ^ M2) * M1;

    return float(n) * (1.0/float(0xffffffffU));
}

float hash_fast(uvec3 q)
{
    q *= uvec3(M1, M2, M3);

    uint n = (q.x ^ q.y ^ q.z) * M1;

    return float(n) * (1.0/float(0xffffffffU));
}

// 2D, but using shifted 2D textures
float noise_2d(vec2 pos) {
    return texture(t_noise, pos).x;
}

// 3D, but using shifted 2D textures
float noise_3d(vec3 pos) {
    pos.z *= 15.0;
    uint z = uint(trunc(pos.z));
    vec2 offs0 = vec2(hash_one(z), hash_one(z + 73u));
    vec2 offs1 = vec2(hash_one(z + 1u), hash_one(z + 1u + 73u));
    return mix(texture(sampler2D(t_noise, s_noise), pos.xy + offs0).x, texture(sampler2D(t_noise, s_noise), pos.xy + offs1).x, fract(pos.z));
}

// 3D version of `snoise`
float snoise3(in vec3 x) {
    uvec3 p = uvec3(floor(x) + 10000.0);
    vec3 f = fract(x);
    //f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(
            mix(hash_fast(p + uvec3(0,0,0)), hash_fast(p + uvec3(1,0,0)), f.x),
            mix(hash_fast(p + uvec3(0,1,0)), hash_fast(p + uvec3(1,1,0)), f.x),
            f.y),
        mix(
            mix(hash_fast(p + uvec3(0,0,1)), hash_fast(p + uvec3(1,0,1)), f.x),
            mix(hash_fast(p + uvec3(0,1,1)), hash_fast(p + uvec3(1,1,1)), f.x),
            f.y),
        f.z);
}

// 4D noise
float snoise(in vec4 x) {
    vec4 p = floor(x);
    vec4 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(
            mix(
                mix(hash(p + vec4(0,0,0,0)), hash(p + vec4(1,0,0,0)), f.x),
                mix(hash(p + vec4(0,1,0,0)), hash(p + vec4(1,1,0,0)), f.x),
                f.y),
            mix(
                mix(hash(p + vec4(0,0,1,0)), hash(p + vec4(1,0,1,0)), f.x),
                mix(hash(p + vec4(0,1,1,0)), hash(p + vec4(1,1,1,0)), f.x),
                f.y),
            f.z),
        mix(
            mix(
                mix(hash(p + vec4(0,0,0,1)), hash(p + vec4(1,0,0,1)), f.x),
                mix(hash(p + vec4(0,1,0,1)), hash(p + vec4(1,1,0,1)), f.x),
                f.y),
            mix(
                mix(hash(p + vec4(0,0,1,1)), hash(p + vec4(1,0,1,1)), f.x),
                mix(hash(p + vec4(0,1,1,1)), hash(p + vec4(1,1,1,1)), f.x),
                f.y),
            f.z),
        f.w);
}

vec3 rand_perm_3(vec3 pos) {
    return abs(sin(pos * vec3(1473.7 * pos.z + 472.3, 8891.1 * pos.x + 723.1, 3813.3 * pos.y + 982.5)));
}

vec4 rand_perm_4(vec4 pos) {
    return sin(473.3 * pos * vec4(317.3 * pos.w + 917.7, 1473.7 * pos.z + 472.3, 8891.1 * pos.x + 723.1, 3813.3 * pos.y + 982.5) / pos.yxwz);
}

vec3 smooth_rand(vec3 pos, float lerp_axis) {
    return vec3(snoise(vec4(pos, lerp_axis)), snoise(vec4(pos + 400.0, lerp_axis)), snoise(vec4(pos + 1000.0, lerp_axis)));
    vec3 r0 = rand_perm_3(vec3(pos.x, pos.y, pos.z) + floor(lerp_axis));
    vec3 r1 = rand_perm_3(vec3(pos.x, pos.y, pos.z) + floor(lerp_axis + 1.0));
    return r0 + (r1 - r0) * fract(lerp_axis);
}
#endif
