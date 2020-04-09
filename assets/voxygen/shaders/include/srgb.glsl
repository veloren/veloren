//https://gamedev.stackexchange.com/questions/92015/optimized-linear-to-srgb-glsl
vec3 srgb_to_linear(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 higher = pow((srgb + vec3(0.055))/vec3(1.055), vec3(2.4));
    vec3 lower = srgb/vec3(12.92);

    return mix(higher, lower, cutoff);
}

// Phong reflection.
//
// Note: norm, dir, light_dir must all be normalizd.
vec3 light_reflection_factor(vec3 norm, vec3 dir, vec3 light_dir, vec3 k_d, vec3 k_s, float alpha) {
    float ndotL = max(dot(norm, -light_dir), 0.0);
    if (ndotL > 0.0/* && dot(s_norm, -light_dir) > 0.0*/) {
        vec3 H = normalize(light_dir + dir);
        // (k_d * (L ⋅ N) + k_s * (R ⋅ V)^α)
        return k_d * ndotL + k_s * pow(max(dot(norm, H), 0.0), alpha * 4.0);
    }
    return vec3(0.0);
}
