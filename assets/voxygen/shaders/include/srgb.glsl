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
    if (cos_wi <= 0.0 || cos_wo <= 0.0) {
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
        (4 * abs(dot_wi_wh)) *
        max(abs(cos_wi), abs(cos_wo)) *
        schlick_fresnel(R_s, dot_wi_wh);
    // Spectrum specular = distribution->D(wh) /
    //     (4 * AbsDot(wi, wh) *
    //      std::max(AbsCosTheta(wi), AbsCosTheta(wo))) *
    //      SchlickFresnel(Dot(wi, wh));
    return mix(/*diffuse*//* + specular*/diffuse + specular, vec3(0.0), bvec3(all(equal(light_dir, dir))));
}

// Phong reflection.
//
// Note: norm, dir, light_dir must all be normalizd.
vec3 light_reflection_factor(vec3 norm, vec3 dir, vec3 light_dir, vec3 k_d, vec3 k_s, float alpha) {
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
    // return vec3(1.0);
    /*if (dot(mu, mu) == 0.0) {
        return vec3(1.0);
    }*//* else {
        return vec3(0.0);
    }*/
    // return vec3(0.0);
    vec3 surface_dir = surface_alt < wpos.z ? vec3(0.0, 0.0, 1.0) : vec3(0.0, 0.0, -1.0);
    // vec3 surface_dir = faceforward(vec3(0.0, 0.0, 1.0), ray_dir, vec3(0.0, 0.0, 1.0));
    bool _intersects_surface = IntersectRayPlane(wpos, ray_dir, vec3(0.0, 0.0, surface_alt), surface_dir, defaultpos);
    float depth = length(defaultpos - wpos);
    return exp(-mu * depth);
}

// Same as compute_attenuation but since both point are known, set a maximum to make sure we don't exceed the length
// from the default point.
vec3 compute_attenuation_point(vec3 wpos, vec3 ray_dir, vec3 mu, float surface_alt, vec3 defaultpos) {
    // return vec3(1.0);
    /*if (dot(mu, mu) == 0.0) {
        return vec3(1.0);
    }*//* else {
        return vec3(0.0);
    }*/
    // return vec3(0.0);
    vec3 surface_dir = surface_alt < wpos.z ? vec3(0.0, 0.0, 1.0) : vec3(0.0, 0.0, -1.0);
    // vec3 surface_dir = faceforward(vec3(0.0, 0.0, 1.0), ray_dir, vec3(0.0, 0.0, 1.0));
    float max_length = dot(defaultpos - wpos, defaultpos - wpos);
    bool _intersects_surface = IntersectRayPlane(wpos, ray_dir, vec3(0.0, 0.0, surface_alt), surface_dir, defaultpos);
    float depth2 = min(max_length, dot(defaultpos - wpos, defaultpos - wpos));
    return exp(-mu * sqrt(depth2));
}
