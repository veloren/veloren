// Linear RGB, attenuation coefficients for water at roughly R, G, B wavelengths.
// See https://en.wikipedia.org/wiki/Electromagnetic_absorption_by_water
const vec3 MU_WATER = vec3(0.6, 0.04, 0.01);

// // NOTE: Automatic in v4.0
// float
// mip_map_level(in vec2 texture_coordinate)
// {
//     // The OpenGL Graphics System: A Specification 4.2
//     //  - chapter 3.9.11, equation 3.21
//
//
//     vec2  dx_vtc        = dFdx(texture_coordinate);
//     vec2  dy_vtc        = dFdy(texture_coordinate);
//     float delta_max_sqr = max(dot(dx_vtc, dx_vtc), dot(dy_vtc, dy_vtc));
//
//
//     //return max(0.0, 0.5 * log2(delta_max_sqr) - 1.0); // == log2(sqrt(delta_max_sqr));
//     return 0.5 * log2(delta_max_sqr); // == log2(sqrt(delta_max_sqr));
// }

//https://gamedev.stackexchange.com/questions/92015/optimized-linear-to-srgb-glsl
vec3 srgb_to_linear(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 higher = pow((srgb + vec3(0.055))/vec3(1.055), vec3(2.4));
    vec3 lower = srgb/vec3(12.92);

    return mix(higher, lower, cutoff);
}

vec3 linear_to_srgb(vec3 col) {
    // bvec3 cutoff = lessThan(col, vec3(0.0060));
    // return mix(11.500726 * col, , cutoff);
    vec3 s1 = vec3(sqrt(col.r), sqrt(col.g), sqrt(col.b));
    vec3 s2 = vec3(sqrt(s1.r), sqrt(s1.g), sqrt(s1.b));
    vec3 s3 = vec3(sqrt(s2.r), sqrt(s2.g), sqrt(s2.b));
    return vec3(
            mix(11.500726 * col.r, (0.585122381 * s1.r + 0.783140355 * s2.r - 0.368262736 * s3.r), clamp((col.r - 0.0060) * 10000.0, 0.0, 1.0)),
            mix(11.500726 * col.g, (0.585122381 * s1.g + 0.783140355 * s2.g - 0.368262736 * s3.g), clamp((col.g - 0.0060) * 10000.0, 0.0, 1.0)),
            mix(11.500726 * col.b, (0.585122381 * s1.b + 0.783140355 * s2.b - 0.368262736 * s3.b), clamp((col.b - 0.0060) * 10000.0, 0.0, 1.0))
    );
}

float pow5(float x) {
    float x2 = x * x;
    return x2 * x2 * x;
}

vec4 pow5(vec4 x) {
    vec4 x2 = x * x;
    return x2 * x2 * x;
}

// Fresnel angle for perfectly specular dialectric materials.

// Schlick approximation
vec3 schlick_fresnel(vec3 Rs, float cosTheta) {
    // auto pow5 = [](Float v) { return (v * v) * (v * v) * v; };
    // return Rs + pow5(1 - cosTheta) * (Spectrum(1.) - Rs);
    return Rs + pow5(1.0 - cosTheta) * (1.0 - Rs);
}

// Beckmann Distribution
float BeckmannDistribution_D(float NdotH, float alpha) {
    const float PI = 3.1415926535897932384626433832795;
    float NdotH2 = NdotH * NdotH;
    float NdotH2m2 = NdotH2 * alpha * alpha;
    float k_spec = exp((NdotH2 - 1) / NdotH2m2) / (PI * NdotH2m2 * NdotH2);
    return mix(k_spec, 0.0, NdotH == 0.0);
}

// Voxel Distribution
float BeckmannDistribution_D_Voxel(vec3 wh, vec3 voxel_norm, float alpha) {
    vec3 sides = sign(voxel_norm);
    // vec3 cos_sides_i = /*sides * */sides * norm;
    // vec3 cos_sides_o = max(sides * view_dir, 0.0);

    vec3 NdotH = wh * sides;//max(wh * sides, 0.0);/*cos_sides_i*///max(sides * wh, 0.0);

    const float PI = 3.1415926535897932384626433832795;
    vec3 NdotH2 = NdotH * NdotH;
    vec3 NdotH2m2 = NdotH2 * alpha * alpha;
    vec3 k_spec = exp((NdotH2 - 1) / NdotH2m2) / (PI * NdotH2m2 * NdotH2);
    return dot(mix(k_spec, /*cos_sides_o*/vec3(0.0), equal(NdotH, vec3(0.0))), /*cos_sides_i*/abs(voxel_norm));
    // // const float PI = 3.1415926535897932384626433832795;
    // const vec3 normals[6] = vec3[](vec3(1,0,0), vec3(0,1,0), vec3(0,0,1), vec3(-1,0,0), vec3(0,-1,0), vec3(0,0,-1));

    // float voxel_norm = 0.0;
    // for (int i = 0; i < 6; i ++) {
    //     // Light reflecting off the half-angle can shine on up to three sides.
    //     // So, the idea here is to figure out the ratio of visibility of each of these
    //     // three sides such that their sum adds to 1, then computing a Beckmann Distribution for each side times
    //     // the this ratio.
    //     //
    //     // The ratio of these normals in each direction should be the sum of their cosines with the light over π,
    //     // I think.
    //     //
    //     // cos (wh, theta)
    //     //
    //     // - one normal
    //     //
    //     // The ratio of each of the three exposed sides should just be the slope.
    //     vec3 side = normals[i];
    //     float side_share = max(dot(norm, side), 0.0);
    //     float NdotH = max(dot(wh, side), 0.0);
    //     voxel_norm += side_share * BeckmannDistribution_D(NdotH, alpha);
    //     // voxel_norm += normals[i] * side_visible * max(dot(-cam_dir, normals[i]), 0.0);
    //     // voxel_norm += normals[i] * side_visible * max(dot(-cam_dir, normals[i]), 0.0);
    // }

    // /* float NdotH = dot(wh, norm);
    // float NdotH2 = NdotH * NdotH;
    // float NdotH2m2 = NdotH2 * alpha * alpha;

    // float k_spec = exp((NdotH2 - 1) / NdotH2m2) / (PI * NdotH2m2 * NdotH2);
    // return mix(k_spec, 0.0, NdotH == 0.0); */
    // return voxel_norm;
}

float TrowbridgeReitzDistribution_D_Voxel(vec3 wh, vec3 voxel_norm, float alpha) {
    vec3 sides = sign(voxel_norm);
    // vec3 cos_sides_i = /*sides * */sides * norm;
    // vec3 cos_sides_o = max(sides * view_dir, 0.0);

    vec3 NdotH = wh * sides;//max(wh * sides, 0.0);/*cos_sides_i*///max(sides * wh, 0.0);

    const float PI = 3.1415926535897932384626433832795;
    vec3 NdotH2 = NdotH * NdotH;
    // vec3 m2 = alpha * alpha;
    // vec3 NdotH2m2 = NdotH2 * m2;
    vec3 NdotH2m2 = NdotH2 * alpha * alpha;
    // vec3 Tan2Theta = (1 - NdotH2) / NdotH2;
    // vec3 e = (NdotH2 / m2 + (1 - NdotH2) / m2) * Tan2Theta;
    // vec3 e = 1 / m2 * (1 - NdotH2) / NdotH2;
    vec3 e = (1 - NdotH2) / NdotH2m2;
    vec3 k_spec = 1.0 / (PI * NdotH2m2 * NdotH2 * (1 + e) * (1 + e));
    // vec3 k_spec = exp((NdotH2 - 1) / NdotH2m2) / (PI * NdotH2m2 * NdotH2);
    return dot(mix(k_spec, /*cos_sides_o*/vec3(0.0), equal(NdotH, vec3(0.0))), /*cos_sides_i*/abs(voxel_norm));
}

float BeckmannDistribution_Lambda(vec3 norm, vec3 dir, float alpha) {
    float CosTheta = /*max(dot(norm, dir), 0.0);*/dot(norm, dir);
    /* if (CosTheta == 0.0) {
        return 0.0;
    }
    float SinTheta = sqrt(1.0 - CosTheta * CosTheta);
    float TanTheta = SinTheta / CosTheta;
    float absTanTheta = abs(TanTheta); */
    // vec3 w = normalize(dir - dot(dir, norm) * (norm));
    // float CosTheta = w.z;
    float SinTheta = sqrt(1.0 - CosTheta * CosTheta);
    float TanTheta = SinTheta / CosTheta;
    float absTanTheta = abs(TanTheta);
    /* if (isinf(absTanTheta)) {
        return 0.0;
    } */
    /* float CosPhi = mix(clamp(projDirNorm.x / sinTheta, -1.0, 1.0), 0.0, sinTheta == 0.0);
    float SinPhi = mix(clamp(projDirNorm.y / sinTheta, -1.0, 1.0), 0.0, sinTheta == 0.0);
    float alpha = sqrt(CosPhi * CosPhi * alphax * alphax + SinPhi * SinPhi * alphay * alphay); */
    // Float absTanTheta = std::abs(TanTheta(w));
    // if (std::isinf(absTanTheta)) return 0.;
    // <<Compute alpha for direction w>>
    //    Float alpha = std::sqrt(Cos2Phi(w) * alphax * alphax +
    //                            Sin2Phi(w) * alphay * alphay);
    float a = 1.0 / (alpha * absTanTheta);
    /* if (a >= 1.6) {
        return 0.0;
    }

    return (1.0 - 1.259 * a + 0.396 * a * a) / (3.535 * a + 2.181 * a * a); */

    return mix(max(0.0, (1.0 - 1.259 * a + 0.396 * a * a) / (3.535 * a + 2.181 * a * a)), 0.0, isinf(absTanTheta) || a >= 1.6);
    // Float a = 1 / (alpha * absTanTheta);
    // if (a >= 1.6f)
    //     return 0;
    // return (1 - 1.259f * a + 0.396f * a * a) /
    //        (3.535f * a + 2.181f * a * a);
    // return 1 / (1 + Lambda(wo) + Lambda(wi));
}

float BeckmannDistribution_G(vec3 norm, vec3 dir, vec3 light_dir, float alpha) {
    // return 1 / (1 + Lambda(wo) + Lambda(wi));
    return 1.0 / (1.0 + BeckmannDistribution_Lambda(norm, dir, alpha) + BeckmannDistribution_Lambda(norm, -light_dir, alpha));
}

// Fresnel blending
//
// http://www.pbr-book.org/3ed-2018/Reflection_Models/Microfacet_Models.html#fragment-MicrofacetDistributionPublicMethods-2
// and
// http://www.pbr-book.org/3ed-2018/Reflection_Models/Fresnel_Incidence_Effects.html
vec3 FresnelBlend_f(vec3 norm, vec3 dir, vec3 light_dir, vec3 R_d, vec3 R_s, float alpha) {
    const float PI = 3.1415926535897932384626433832795;
    alpha = alpha * sqrt(2.0);
    float cos_wi = /*max(*/dot(-light_dir, norm)/*, 0.0)*/;
    float cos_wo = /*max(*/dot(dir, norm)/*, 0.0)*/;

    vec3 diffuse = (28.0 / (23.0 * PI)) * R_d *
        (1.0 - R_s) *
        (1.0 - pow5(1.0 - 0.5 * abs(cos_wi))) *
        (1.0 - pow5(1.0 - 0.5 * abs(cos_wo)));
    /* Spectrum diffuse = (28.f/(23.f*Pi)) * Rd *
        (Spectrum(1.f) - Rs) *
        (1 - pow5(1 - .5f * AbsCosTheta(wi))) *
        (1 - pow5(1 - .5f * AbsCosTheta(wo))); */
    // Vector3f wh = wi + wo;
    vec3 wh = -light_dir + dir;
#if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
    bool is_blocked = cos_wi == 0.0 || cos_wo == 0.0;
#else
    bool is_blocked = cos_wi <= 0.0 || cos_wo <= 0.0;
#endif
    if (is_blocked) {
        return vec3(/*diffuse*/0.0);
    }
    // if (cos_wo < 0.0) {
    //     return /*vec3(0.0)*/diffuse;
    // }
    /* if (cos_wi == 0.0 || cos_wo == 0.0) {
        return vec3(0.0);
    } */
    /* if (wh.x == 0 && wh.y == 0 && wh.z == 0) {
        return vec3(0.0);
        // return Spectrum(0);
    } */
    wh = normalize(wh);//mix(normalize(wh), vec3(0.0), equal(light_dir, dir));
    float dot_wi_wh = dot(-light_dir, wh);
    vec3 specular = BeckmannDistribution_D(dot(wh, norm), alpha) /
        (4 * abs(dot_wi_wh) *
        max(abs(cos_wi), abs(cos_wo))) *
        schlick_fresnel(R_s, dot_wi_wh);
    // Spectrum specular = distribution->D(wh) /
    //     (4 * AbsDot(wi, wh) *
    //      std::max(AbsCosTheta(wi), AbsCosTheta(wo))) *
    //      SchlickFresnel(Dot(wi, wh));
    return mix(/*diffuse*//* + specular*/diffuse + specular, vec3(0.0), bvec3(all(equal(light_dir, dir))));
}

// Fresnel blending
//
// http://www.pbr-book.org/3ed-2018/Reflection_Models/Microfacet_Models.html#fragment-MicrofacetDistributionPublicMethods-2
// and
// http://www.pbr-book.org/3ed-2018/Reflection_Models/Fresnel_Incidence_Effects.html
vec3 FresnelBlend_Voxel_f(vec3 norm, vec3 dir, vec3 light_dir, vec3 R_d, vec3 R_s, float alpha, vec3 voxel_norm, float dist) {
    const float PI = 3.1415926535897932384626433832795;
    alpha = alpha * sqrt(2.0);
    float cos_wi = /*max(*/dot(-light_dir, norm)/*, 0.0)*/;
    float cos_wo = /*max(*/dot(dir, norm)/*, 0.0)*/;

#if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
    vec4 AbsNdotL = abs(vec4(light_dir, cos_wi));
    vec4 AbsNdotV = abs(vec4(dir, cos_wo));
#else
    vec3 sides = sign(voxel_norm);
    vec4 AbsNdotL = vec4(max(-light_dir * sides, 0.0), abs(cos_wi));
    vec4 AbsNdotV = vec4(max(dir * sides, 0.0), abs(cos_wo));
#endif

    // float R_r = 1.0 - R_s;
    // float R_r = 1.0 - schlick_fresnel(R_s, cos_wi);
    // // Rs + pow5(1.0 - cosTheta) * (1.0 - Rs)
    // vec4 R_r = 1.0 - (R_s + (1.0 - R_s) * schlick_fresnel(R_s, cos_wi));
    // mat4 R_r = 1.0 - (vec4(R_s, 0.0) + vec4(1.0 - R_s, 0.0) * pow5(1.0 - AbsNdotL));
    // vec4 AbsNdotL5 = pow5(1.0 - AbsNdotL);
    // vec4 R_s4 = vec4(R_s, 0.0);
    // mat4 R_r =
    //     // mat4(1.0 - (R_s.r + (1.0 - R_s.r) * AbsNdotL5),
    //     //      1.0 - (R_s.g + (1.0 - R_s.g) * AbsNdotL5),
    //     //      1.0 - (R_s.b + (1.0 - R_s.b) * AbsNdotL5),
    //     //      vec4(0.0)
    //     //     );
    //     mat4(1.0 - (R_s4 + (1.0 - R_s4) * AbsNdotL5.x),
    //          1.0 - (R_s4 + (1.0 - R_s4) * AbsNdotL5.y),
    //          1.0 - (R_s4 + (1.0 - R_s4) * AbsNdotL5.z),
    //          1.0 - (R_s4 + (1.0 - R_s4) * AbsNdotL5.w)
    //         );
    // * ) (R1.0 - R_s.r) 1.0 - (vec4(R_s, 0.0) + vec4(1.0 - R_s, 0.0) * pow5(1.0 - AbsNdotL));

    vec4 diffuse_factor =
        // vec4(abs(vec4(-light_dir * sides, cos_wi)))
        (1.0 - pow5(1.0 - 0.5 * AbsNdotL)) *
        // (1.0 - pow5(1.0 - 0.5 * abs(vec4(-light_dir * sides, cos_wi)))) *
        // (1.0 - pow5(1.0 - 0.5 * abs(vec4(dir * sides, cos_wo))))
        (1.0 - pow5(1.0 - 0.5 * AbsNdotV))
        // vec4(1.0)
        ;
    /* vec4 diffuse_factor =
        (1.0 - pow5(1.0 - 0.5 * max(vec4(-light_dir * sides, abs(cos_wi)), 0.0))) *
        (1.0 - pow5(1.0 - 0.5 * max(vec4(dir * sides, abs(cos_wo)), 0.0))); */

    vec3 diffuse = (28.0 / (23.0 * PI))/*(1.0 / PI)*/ * R_d *
        (1.0 - R_s) *
        //vec3(
        dot(diffuse_factor, /*R_r * */vec4(abs(norm) * (1.0 - dist), dist))
        //)
        ;

    vec3 wh = -light_dir + dir;
#if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
    bool is_blocked = cos_wi == 0.0 || cos_wo == 0.0;
#else
    bool is_blocked = cos_wi <= 0.0 || cos_wo <= 0.0;
#endif
    if (is_blocked) {
        return vec3(/*diffuse*/0.0);
    }
    wh = normalize(wh);//mix(normalize(wh), vec3(0.0), equal(light_dir, dir));
    float dot_wi_wh = dot(-light_dir, wh);
    // float distr = TrowbridgeReitzDistribution_D_Voxel(wh, voxel_norm, alpha);
    float distr = BeckmannDistribution_D_Voxel(wh, voxel_norm, alpha);
    // float distr = BeckmannDistribution_D(dot(wh, norm), alpha);
    vec3 specular = distr /
        (4 * abs(dot_wi_wh) *
        max(abs(cos_wi), abs(cos_wo))) *
        schlick_fresnel(R_s, dot_wi_wh);
    return mix(/*diffuse*//* + specular*/diffuse + specular, vec3(0.0), bvec3(all(equal(light_dir, dir))));
}

// Phong reflection.
//
// Note: norm, dir, light_dir must all be normalizd.
vec3 light_reflection_factor2(vec3 norm, vec3 dir, vec3 light_dir, vec3 k_d, vec3 k_s, float alpha) {
    // TODO: These are supposed to be the differential changes in the point location p, in tangent space.
    // That is, assuming we can parameterize a 2D surface by some function p : R² → R³, mapping from
    // points in a plane to 3D points on the surface, we can define
    // ∂p(u,v)/∂u and ∂p(u,v)/∂v representing the changes in the pont location as we move along these
    // coordinates.
    //
    // Then we can define the normal at a point, n(u,v) = ∂p(u,v)/∂u × ∂p(u,v)/∂v.
    //
    // Additionally, we can define the change in *normals* at each point using the
    // Weingarten equations (see http://www.pbr-book.org/3ed-2018/Shapes/Spheres.html):
    //
    // ∂n/∂u = (fF - eG) / (EG - F²) ∂p/∂u + (eF - fE) / (EG - F²) ∂p/∂v
    // ∂n/∂v = (gF - fG) / (EG - F²) ∂p/∂u + (fF - gE) / (EG - F²) ∂p/∂v
    //
    // where
    //
    // E = |∂p/∂u ⋅ ∂p/∂u|
    // F = ∂p/∂u ⋅ ∂p/∂u
    // G = |∂p/∂v ⋅ ∂p/∂v|
    //
    // and
    //
    // e = n ⋅ ∂²p/∂u²
    // f = n ⋅ ∂²p/(∂u∂v)
    // g = n ⋅ ∂²p/∂v²
    //
    // For planes (see http://www.pbr-book.org/3ed-2018/Shapes/Triangle_Meshes.html) we have
    // e = f = g = 0 (since the plane has no curvature of any sort) so we get:
    //
    // ∂n/∂u = (0, 0, 0)
    // ∂n/∂v = (0, 0, 0)
    //
    // To find ∂p/∂u and ∂p/∂v, we first write p and u parametrically:
    //    p(u, v) = p0 + u ∂p/∂u + v ∂p/∂v
    //
    // ( u₀ - u₂    v₀ - v₂
    //   u₁ - u₂    v₁ - v₂ )
    //
    // Basis: plane norm = norm = (0, 0, 1), x vector = any orthgonal vector on the plane.
    // vec3 w_i =
    // vec3 w_i = vec3(view_mat * vec4(-light_dir, 1.0));
    // vec3 w_o = vec3(view_mat * vec4(light_dir, 1.0));
    float g = 1.0;// BeckmannDistribution_G(norm, dir, light_dir, alpha);
    return FresnelBlend_f(norm, dir, light_dir, k_d/* * max(dot(norm, -light_dir), 0.0)*/, k_s * g, alpha);
    // const float PI = 3.141592;
    // alpha = alpha * sqrt(2.0);
    // float ndotL = /*max*/(dot(norm, -light_dir)/*, 0.0*/);

    // //if (ndotL > 0.0/* && dot(s_norm, -light_dir) > 0.0*/) {
    //     vec3 H = normalize(-light_dir + dir);

    //     float NdotH = dot(norm, H);
    //     float NdotH2 = NdotH * NdotH;
    //     float NdotH2m2 = NdotH2 * alpha * alpha;
    //     float k_spec = exp((NdotH2 - 1) / NdotH2m2) / (PI * NdotH2m2 * NdotH2);
    //     return mix(k_s * k_spec, vec3(0.0), bvec3(ndotL <= 0.0 || NdotH == 0.0));
    //     //
    //     // (k_d * (L ⋅ N) + k_s * (R ⋅ V)^α)
    //     // return k_d * ndotL + mix(k_s * pow(max(dot(norm, H), 0.0), alpha * 4.0), vec3(0.0), bvec3(ndotL == 0.0));
    // // }
    // // return vec3(0.0);
}

vec3 light_reflection_factor(vec3 norm, vec3 dir, vec3 light_dir, vec3 k_d, vec3 k_s, float alpha, vec3 voxel_norm, float voxel_lighting) {
#if (LIGHTING_ALGORITHM == LIGHTING_ALGORITHM_LAMBERTIAN)
    const float PI = 3.141592;
    #if (LIGHTING_DISTRIBUTION_SCHEME == LIGHTING_DISTRIBUTION_SCHEME_VOXEL)
        #if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
    vec4 AbsNdotL = abs(vec4(light_dir, dot(norm, light_dir)));
        #else
    vec3 sides = sign(voxel_norm);
    vec4 AbsNdotL = max(vec4(-light_dir * sides, dot(norm, -light_dir)), 0.0);
        #endif
    float diffuse = dot(AbsNdotL, vec4(abs(voxel_norm) * (1.0 - voxel_lighting), voxel_lighting));
    #elif (LIGHTING_DISTRIBUTION_SCHEME == LIGHTING_DISTRIBUTION_SCHEME_MICROFACET)
        #if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
    float diffuse = abs(dot(norm, light_dir));
        #else
    float diffuse = max(dot(norm, -light_dir), 0.0);
        #endif
    #endif
    return k_d / PI * diffuse;
#elif (LIGHTING_ALGORITHM == LIGHTING_ALGORITHM_BLINN_PHONG)
    const float PI = 3.141592;
    alpha = alpha * sqrt(2.0);
    #if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
    float ndotL = abs(dot(norm, light_dir));
    #else
    float ndotL = max(dot(norm, -light_dir), 0.0);
    #endif

    if (ndotL > 0.0) {
    #if (LIGHTING_DISTRIBUTION_SCHEME == LIGHTING_DISTRIBUTION_SCHEME_VOXEL)
        #if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
        vec4 AbsNdotL = abs(vec4(light_dir, ndotL));
        #else
        vec3 sides = sign(voxel_norm);
        vec4 AbsNdotL = max(vec4(-light_dir * sides, ndotL), 0.0);
        #endif
        float diffuse = dot(AbsNdotL, vec4(abs(voxel_norm) * (1.0 - voxel_lighting), voxel_lighting));
    #elif (LIGHTING_DISTRIBUTION_SCHEME == LIGHTING_DISTRIBUTION_SCHEME_MICROFACET)
        float diffuse = ndotL;
    #endif
        vec3 H = normalize(-light_dir + dir);

    #if (LIGHTING_TYPE & LIGHTING_TYPE_TRANSMISSION) != 0
        float NdotH = abs(dot(norm, H));
    #else
        float NdotH = max(dot(norm, H), 0.0);
    #endif
        return (1.0 - k_s) / PI * k_d * diffuse + k_s * pow(NdotH, alpha/* * 4.0*/);
    }

    return vec3(0.0);
#elif (LIGHTING_ALGORITHM == LIGHTING_ALGORITHM_ASHIKHMIN)
    #if (LIGHTING_DISTRIBUTION_SCHEME == LIGHTING_DISTRIBUTION_SCHEME_VOXEL)
        return FresnelBlend_Voxel_f(norm, dir, light_dir, k_d/* * max(dot(norm, -light_dir), 0.0)*/, k_s, alpha, voxel_norm, voxel_lighting);
    #elif (LIGHTING_DISTRIBUTION_SCHEME == LIGHTING_DISTRIBUTION_SCHEME_MICROFACET)
    //if (voxel_lighting < 1.0) {
        return FresnelBlend_f(norm, dir, light_dir, k_d/* * max(dot(norm, -light_dir), 0.0)*/, k_s, alpha);
    //} else {
    //    return FresnelBlend_f(norm, dir, light_dir, k_d/* * max(dot(norm, -light_dir), 0.0)*/, k_s, alpha);
    //}
    #endif
#endif
}

float rel_luminance(vec3 rgb)
{
    // https://en.wikipedia.org/wiki/Relative_luminance
    const vec3 W = vec3(0.2126, 0.7152, 0.0722);
    return dot(rgb, W);
}

// From https://discourse.vvvv.org/t/infinite-ray-intersects-with-infinite-plane/10537
// out of laziness.
bool IntersectRayPlane(vec3 rayOrigin, vec3 rayDirection, vec3 posOnPlane, vec3 planeNormal, inout vec3 intersectionPoint)
{
  float rDotn = dot(rayDirection, planeNormal);

  //parallel to plane or pointing away from plane?
  if (rDotn < 0.0000001 )
    return false;

  float s = dot(planeNormal, (posOnPlane - rayOrigin)) / rDotn;

  intersectionPoint = rayOrigin + s * rayDirection;

  return true;
}

// Compute uniform attenuation due to beam passing through a substance that fills an area below a horizontal plane
// (e.g. in most cases, water below the water surface depth) using the simplest form of the Beer-Lambert law
// (https://en.wikipedia.org/wiki/Beer%E2%80%93Lambert_law):
//
// I(z) = I₀ e^(-μz)
//
// We compute this value, except for the initial intensity which may be multiplied out later.
//
// wpos is the position of the point being hit.
// ray_dir is the reversed direction of the ray (going "out" of the point being hit).
// mu is the attenuation coefficient for R, G, and B wavelenghts.
// surface_alt is the estimated altitude of the horizontal surface separating the substance from air.
// defaultpos is the position to use in computing the distance along material at this point if there was a failure.
//
// Ideally, defaultpos is set so we can avoid branching on error.
vec3 compute_attenuation(vec3 wpos, vec3 ray_dir, vec3 mu, float surface_alt, vec3 defaultpos) {
#if (LIGHTING_TRANSPORT_MODE == LIGHTING_TRANSPORT_MODE_IMPORTANCE)
    return vec3(1.0);
#elif (LIGHTING_TRANSPORT_MODE == LIGHTING_TRANSPORT_MODE_RADIANCE)
    // return vec3(1.0);
    /*if (mu == vec3(0.0)) {
        return vec3(1.0);
    }*//* else {
        return vec3(0.0);
    }*/
    // return vec3(0.0);
    // vec3 surface_dir = /*surface_alt < wpos.z ? vec3(0.0, 0.0, -1.0) : vec3(0.0, 0.0, 1.0)*/vec3(0.0, 0.0, sign(surface_alt - wpos.z));
    vec3 surface_dir = surface_alt < wpos.z ? vec3(0.0, 0.0, -1.0) : vec3(0.0, 0.0, 1.0);
    // vec3 surface_dir = faceforward(vec3(0.0, 0.0, 1.0), ray_dir, vec3(0.0, 0.0, 1.0));
    bool _intersects_surface = IntersectRayPlane(wpos, ray_dir, vec3(0.0, 0.0, surface_alt), surface_dir, defaultpos);
    float depth = length(defaultpos - wpos);
    return exp(-mu * depth);
#endif
}

vec3 compute_attenuation2(vec3 wpos, vec3 ray_dir, vec3 mu, float surface_alt, vec3 defaultpos) {
#if (LIGHTING_TRANSPORT_MODE == LIGHTING_TRANSPORT_MODE_IMPORTANCE)
    return vec3(1.0);
#elif (LIGHTING_TRANSPORT_MODE == LIGHTING_TRANSPORT_MODE_RADIANCE)
    // return vec3(1.0);
    /*if (mu == vec3(0.0)) {
        return vec3(1.0);
    }*//* else {
        return vec3(0.0);
    }*/
    // return vec3(0.0);
    // vec3 surface_dir = /*surface_alt < wpos.z ? vec3(0.0, 0.0, -1.0) : vec3(0.0, 0.0, 1.0)*/vec3(0.0, 0.0, sign(surface_alt - wpos.z));
    vec3 surface_dir = surface_alt < wpos.z ? vec3(0.0, 0.0, 1.0) : vec3(0.0, 0.0, -1.0);
    // vec3 surface_dir = faceforward(vec3(0.0, 0.0, 1.0), ray_dir, vec3(0.0, 0.0, 1.0));
    bool _intersects_surface = IntersectRayPlane(wpos, ray_dir, vec3(0.0, 0.0, surface_alt), surface_dir, defaultpos);
    float depth = length(defaultpos - wpos);
    return exp(-mu * depth);
#endif
}

// Same as compute_attenuation but since both point are known, set a maximum to make sure we don't exceed the length
// from the default point.
vec3 compute_attenuation_point(vec3 wpos, vec3 ray_dir, vec3 mu, float surface_alt, vec3 defaultpos) {
#if (LIGHTING_TRANSPORT_MODE == LIGHTING_TRANSPORT_MODE_IMPORTANCE)
    return vec3(1.0);
#elif (LIGHTING_TRANSPORT_MODE == LIGHTING_TRANSPORT_MODE_RADIANCE)
    // return vec3(1.0);
    /*if (mu == vec3(0.0)) {
        return vec3(1.0);
    }*//* else {
        return vec3(0.0);
    }*/
    // return vec3(0.0);
    vec3 surface_dir = /*surface_alt < wpos.z ? vec3(0.0, 0.0, -1.0) : vec3(0.0, 0.0, 1.0)*/vec3(0.0, 0.0, sign(wpos.z - surface_alt));
    // vec3 surface_dir = surface_alt < wpos.z ? vec3(0.0, 0.0, 1.0) : vec3(0.0, 0.0, -1.0);
    // vec3 surface_dir = faceforward(vec3(0.0, 0.0, 1.0), ray_dir, vec3(0.0, 0.0, 1.0));
    float max_length = dot(defaultpos - wpos, defaultpos - wpos);
    bool _intersects_surface = IntersectRayPlane(wpos, ray_dir, vec3(0.0, 0.0, surface_alt), surface_dir, defaultpos);
    float depth2 = min(max_length, dot(defaultpos - wpos, defaultpos - wpos));
    return exp(-mu * sqrt(depth2));
#endif
}

//#ifdef HAS_SHADOW_MAPS
//    #if (SHADOW_MODE == SHADOW_MODE_MAP)
//uniform sampler2DShadow t_directed_shadow_maps;
//// uniform sampler2DArrayShadow t_directed_shadow_maps;
//
//float ShadowCalculationDirected(in vec4 /*light_pos[2]*/sun_pos, uint lightIndex)
//{
//    float bias = 0.0;//-0.0001;// 0.05 / (2.0 * view_distance.x);
//    // const vec3 sampleOffsetDirections[20] = vec3[]
//    // (
//    //    vec3( 1,  1,  1), vec3( 1, -1,  1), vec3(-1, -1,  1), vec3(-1,  1,  1),
//    //    vec3( 1,  1, -1), vec3( 1, -1, -1), vec3(-1, -1, -1), vec3(-1,  1, -1),
//    //    vec3( 1,  1,  0), vec3( 1, -1,  0), vec3(-1, -1,  0), vec3(-1,  1,  0),
//    //    vec3( 1,  0,  1), vec3(-1,  0,  1), vec3( 1,  0, -1), vec3(-1,  0, -1),
//    //    vec3( 0,  1,  1), vec3( 0, -1,  1), vec3( 0, -1, -1), vec3( 0,  1, -1)
//    //    // vec3(0, 0, 0)
//    // );
//    /* if (lightIndex >= light_shadow_count.z) {
//        return 1.0;
//    } */
//    // vec3 fragPos = sun_pos.xyz;// / sun_pos.w;//light_pos[lightIndex].xyz;
//    float visibility = textureProj(t_directed_shadow_maps, sun_pos);
//    // float visibility = textureProj(t_directed_shadow_maps, vec4(fragPos.xy, /*lightIndex, */fragPos.z + bias, sun_pos.w));
//    return visibility;
//    // return mix(visibility, 0.0, sun_pos.z < -1.0);
//    // return mix(mix(0.0, 1.0, visibility == 1.0), 1.0, sign(sun_pos.w) * sun_pos.z > /*1.0*/abs(sun_pos.w));
//    // return visibility == 1.0 ? 1.0 : 0.0;
//    /* if (visibility == 1.0) {
//        return 1.0;
//    } */
//    // return visibility;
//    /* if (fragPos.z > 1.0) {
//        return 1.0;
//    } */
//    // if (visibility <= 0.75) {
//    //     return 0.0;
//    // }
//    // int samples  = 20;
//    // float shadow = 0.0;
//    // // float bias   = 0.0001;
//    // float viewDistance = length(cam_pos.xyz - fragPos);
//    // // float diskRadius = 0.2 * (1.0 + (viewDistance / screen_res.w)) / 25.0;
//    // float diskRadius = 0.0008;//0.005;// / (2.0 * view_distance.x);//(1.0 + (viewDistance / screen_res.w)) / 25.0;
//    // for(int i = 0; i < samples; ++i)
//    // {
//    //     vec3 currentDepth = fragPos + vec3(sampleOffsetDirections[i].xyz) * diskRadius + bias;
//    //     visibility = texture(t_directed_shadow_maps, vec4(currentDepth.xy, lightIndex, currentDepth.z)/*, -2.5*/);
//    //     shadow += mix(visibility, 1.0, visibility >= 0.5);
//    // }
//    // shadow /= float(samples);
//    // return shadow;
//}
//    #elif (SHADOW_MODE == SHADOW_MODE_NONE || SHADOW_MODE == SHADOW_MODE_CHEAP)
//float ShadowCalculationDirected(in vec4 light_pos[2], uint lightIndex)
//{
//    return 1.0;
//}
//    #endif
//#else
//float ShadowCalculationDirected(in vec4 light_pos[2], uint lightIndex)
//{
//    return 1.0;
//}
//#endif
