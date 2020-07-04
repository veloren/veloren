#version 330 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#if (FLUID_MODE == FLUID_MODE_CHEAP)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE == FLUID_MODE_SHINY)
#define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

// #define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_VOXEL
#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#include <globals.glsl>
#include <sky.glsl>
#include <lod.glsl>

in vec3 f_pos;
in vec3 f_norm;
// in vec2 v_pos_orig;
// in vec4 f_shadow;
// in vec4 f_square;

out vec4 tgt_color;

/// const vec4 sun_pos = vec4(0);
// const vec4 light_pos[2] = vec4[](vec4(0), vec4(0)/*, vec3(00), vec3(0), vec3(0), vec3(0)*/);

#include <sky.glsl>

void main() {
    // tgt_color = vec4(vec3(1.0), 1.0);
    // return;
    // vec3 f_pos = lod_pos(f_pos.xy);
    // vec3 f_col = lod_col(f_pos.xy);

    // vec4 vert_pos4 = view_mat * vec4(f_pos, 1.0);
    // vec3 view_dir = normalize(-vec3(vert_pos4)/* / vert_pos4.w*/);

    float my_alt = /*f_pos.z;*/alt_at_real(f_pos.xy);
    // vec3 f_pos = vec3(f_pos.xy, max(my_alt, f_pos.z));
    /* gl_Position =
        proj_mat *
        view_mat *
        vec4(f_pos, 1);
    gl_Position.z = -1000.0 / (gl_Position.z + 10000.0); */
    vec3 my_pos = vec3(f_pos.xy, my_alt);
    vec3 my_norm = lod_norm(f_pos.xy/*, f_square*/);

    float which_norm = dot(my_norm, normalize(cam_pos.xyz - my_pos));
    // which_norm = 0.5 + which_norm * 0.5;

    // which_norm = pow(max(0.0, which_norm), /*0.03125*/1 / 8.0);// * 0.5;
    // smoothstep
    which_norm = which_norm * which_norm * (3 - 2 * abs(which_norm));

    // which_norm = mix(0.0, 1.0, which_norm > 0.0);
    // vec3 normals[6] = vec3[](vec3(-1,0,0), vec3(1,0,0), vec3(0,-1,0), vec3(0,1,0), vec3(0,0,-1), vec3(0,0,1));
    vec3 f_norm = mix(faceforward(f_norm, cam_pos.xyz - f_pos, -f_norm), my_norm, which_norm);
    vec3 f_pos = mix(f_pos, my_pos, which_norm);
    // vec3 fract_pos = fract(f_pos);
    /* if (length(f_pos - cam_pos.xyz) <= view_distance.x + 32.0) {
        vec4 new_f_pos;
        float depth = 10000000.0;
        vec4 old_coord = all_mat * vec4(f_pos.xyz, 1.0);
        for (int i = 0; i < 6; i ++) {
            // vec4 square = focus_pos.xy + vec4(splay(pos - vec2(1.0, 1.0), splay(pos + vec2(1.0, 1.0))));
            vec3 my_f_norm = normals[i];
            vec3 my_f_tan = normals[(i + 2) % 6];
            vec3 my_f_bitan = normals[(i + 4) % 6];
            mat4 foo = mat4(vec4(my_f_tan, 0), vec4(my_f_bitan, 0), vec4(my_f_norm, 0), vec4(0, 0, 0, 1));
            mat4 invfoo = foo * inverse(foo * all_mat);
            vec4 my_f_pos = invfoo * (old_coord);//vec4(f_pos, 1.0);
            vec4 my_f_proj = all_mat * my_f_pos;
            if (my_f_proj.z <= depth) {
                new_f_pos = my_f_pos;
                f_norm = my_f_norm;
                depth = my_f_proj.z;
            }
        }
        // f_pos = new_f_pos.xyz;
    } */

    // Test for distance to all 6 sides of the enclosing cube.
    // if (/*any(lessThan(fract(f_pos.xy), 0.01))*/fract_pos.x <= 0.1) {
    //     f_norm = faceforward(vec3(-1, 0, 0), f_norm, vec3(1, 0, 0));
    //     f_tan = vec3(0, 1, 0);
    // } else if (fract_pos.y <= 0.1) {
    //     f_norm = faceforward(vec3(0, -1, 0), f_norm, vec3(0, 1, 0));
    //     f_tan = vec3(0, 0, 1);
    // } else {
    //     f_norm = faceforward(vec3(0, 0, -1), f_norm, vec3(0, 0, 1));
    //     f_tan = vec3(1, 0, 0);
    // }
    // vec3 f_bitan = cross(f_norm, f_tan);

    // mat4 foo = mat4(vec4(f_tan, 0), vec4(f_bitan, 0), vec4(f_norm, 0), vec4(0, 0, 0, 1));
    // mat4 invfoo = foo * inverse(foo * all_mat);
    // vec3 old_coord = all_mat * vec4(f_pos.xyz, 1.0);
    // vec4 new_f_pos = invfoo * (old_coord);//vec4(f_pos, 1.0);
    vec3 f_col = lod_col(f_pos.xy);
    // tgt_color = vec4(f_col, 1.0);
    // return;
    // vec3 f_col = srgb_to_linear(vec3(1.0));
    // vec3 f_norm = faceforward(f_norm, cam_pos.xyz - f_pos, -f_norm);
    // vec3 f_up = faceforward(cam_pos.xyz - f_pos, vec3(0.0, 0.0, -1.0), cam_pos.xyz - f_pos);
    // vec3 f_norm = faceforward(f_norm, /*vec3(cam_pos.xyz - f_pos.xyz)*/vec3(0.0, 0.0, -1.0), f_norm);

    // const vec3 normals[3] = vec3[](vec3(1,0,0), vec3(0,1,0), vec3(0,0,1));//, vec3(-1,0,0), vec3(0,-1,0), vec3(0,0,-1));
    // const mat3 side_norms = vec3(1, 0, 0), vec3(0, 1, 0), vec3(0, 0, 1);
    // mat3 sides = mat3(
    //     /*vec3(1, 0, 0),
    //     vec3(0, 1, 0),
    //     vec3(0, 0, 1)*/
    //     vec3(1, 0, 0),
    //     // faceforward(vec3(1, 0, 0), -f_norm, vec3(1, 0, 0)),
    //     vec3(0, 1, 0),
    //     // faceforward(vec3(0, 1, 0), -f_norm, vec3(0, 1, 0)),
    //     vec3(0, 0, 1)
    //     // faceforward(vec3(0, 0, 1), -f_norm, vec3(0, 0, 1))
    // );

    // This vector is shorthand for a diagonal matrix, which works because:
    // (1) our voxel normal vectors are exactly the basis vectors in worldspace;
    // (2) only 3 of them can be in the direction of the actual normal anyway.
    // (NOTE: This normal should always be pointing up, so implicitly sides.z = 1.0).
    // vec3 sides = sign(f_norm);
    // // NOTE: Should really be sides * f_norm, i.e. abs(f_norm), but voxel_norm would then re-multiply by sides so it cancels out.
    // vec3 cos_sides_i = sides * f_norm;
    // vec3 cos_sides_o = sides * view_dir;
    // // vec3 side_factor_i = cos_sides_i;
    // // vec3 side_factor_i = f_norm;
    // // vec3 side_factor_i = cos_sides_o;
    // vec3 side_factor_i = 1.0 - pow(1.0 - 0.5 * cos_sides_i, vec3(5));
    // // vec3 side_factor_i = /*abs*/sign(f_norm) * cos_sides_i;//max(cos_sides_i, 0.0);// 1.0 - pow(1.0 - 0.5 * cos_sides_i, vec3(5.0)); // max(sides * f_norm, vec3(0.0));//
    // // vec3 side_factor_i = /*abs*/sign(f_norm) * cos_sides_i;//max(cos_sides_i, 0.0);// 1.0 - pow(1.0 - 0.5 * cos_sides_i, vec3(5.0)); // max(sides * f_norm, vec3(0.0));//
    // // vec3 side_factor_o = max(cos_sides_o, 0.0);// 1.0 - pow(1.0 - 0.5 * max(cos_sides_o, 0.0), vec3(5));
    // vec3 side_factor_o = 1.0 - pow(1.0 - 0.5 * max(cos_sides_o, 0.0), vec3(5));
    // // vec3 side_factor_o = max(cos_sides_o, 0.0);// 1.0 - pow(1.0 - 0.5 * max(cos_sides_o, vec3(0.0)), vec3(5.0));//max(sides * view_dir/* * sign(cos_sides_i) */, vec3(0.0));
    // // vec3 side_factor_o = max(sides * view_dir/* * cos_sides_o*/, 0.0);// 1.0 - pow(1.0 - 0.5 * max(cos_sides_o, vec3(0.0)), vec3(5.0));//max(sides * view_dir/* * sign(cos_sides_i) */, vec3(0.0));
    // // NOTE: side = transpose(sides), so we avoid the extra operatin.
    // // We multply the vector by the matrix from the *left*, so each normal gets multiplied by the corresponding factor.
    // // vec3 voxel_norm = normalize(/*sides * *//*sqrt(1.0 - cos_sides_i * cos_sides_i)*/(side_factor_i * side_factor_o));
    // vec3 voxel_norm = normalize(/*sides * *//*sqrt(1.0 - cos_sides_i * cos_sides_i)*/((28.0 / (23.0 * PI)) * side_factor_i * side_factor_o * sides));
    // vec3 voxel_norm = normalize(sign(f_norm) * sqrt(abs(f_norm)) * max(sign(f_norm) * view_dir, 0.0));
    float f_ao = 1.0;//1.0;//sqrt(dot(cos_sides_i, cos_sides_i) / 3.0);
    // float f_ao = 0.2;
    // sqrt(dot(sqrt(1.0 - cos_sides_i * cos_sides_i)), 1.0 - cos_sides_o/* * cos_sides_o*/);// length(sqrt(1.0 - cos_sides_o * cos_sides_o) / cos_sides_i * cos_sides_o);
    // f_ao = f_ao * f_ao;

    // /* vec3 voxel_norm = vec3(0.0);
    // for (int i = 0; i < 3; i ++) {
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
    //     side = faceforward(side, -f_norm, side);
    //     float cos_wi = max(dot(f_norm, side), 0.0);
    //     float cos_wo = max(dot(view_dir, side), 0.0);
    //     float share = cos_wi * cos_wo;
    //     // float share = (1.0 - pow5(1.0 - 0.5 * cos_wi)) * (1.0 - pow5(1.0 - 0.5 * cos_wo));
    //     voxel_norm += share * side;
    //     // voxel_norm += normals[i] * side_visible * max(dot(-cam_dir, normals[i]), 0.0);
    //     // voxel_norm += normals[i] * side_visible * max(dot(-cam_dir, normals[i]), 0.0);
    // }
    // voxel_norm = normalize(voxel_norm); */

    float dist_lerp = 0.0;//clamp(pow(max(distance(focus_pos.xy, f_pos.xy) - view_distance.x, 0.0) / 4096.0, 2.0), 0, 1);
    // dist_lerp = 0.0;
    // voxel_norm = normalize(mix(voxel_norm, f_norm, /*pow(dist_lerp, 1.0)*/dist_lerp));

    // IDEA:
    // We can represent three faces as sign(voxel_norm).
    vec3 sides = sign(f_norm);
    // There are three relevant vectors: normal, tangent, and bitangent.
    // We say normal is the z component, tangent the x component, bitangent the y.
    // A blocking side is in the reverse direction of each.
    // So -sides is the *direction* of the next block.
    // Now, we want to multiply this by the *distance* to the nearest integer in that direction.
    // If sides.x is -1, the direction is 1, so the distance is 1.0 - fract(f_pos.x) and the delta is 1.0 - fract(f_pos.x).
    // If sides.x is 1, the direction is -1, so the distance is fract(f_pos.x) and the delta is -fract(f_pos.x) = 1.0 + fract(-f_pos.x).
    // If sides.x is 0, the direction is 0, so the distance is 0.0 and the delta is 0.0 = 0.0 + fract(0.0 * f_pos.x).
    // (we ignore f_pos < 0 for the time being).
    // Then this is 1.0 + sides.x * fract(-sides.x * f_pos.x);
    // We repeat this for y.
    //
    // We treat z as the dependent variable.
    // IF voxel_norm.x > 0.0, z should increase by voxel_norm.z / voxel_norm.x * delta_sides.x in the x direction;
    // IF voxel_norm.y > 0.0, z should increase by voxel_norm.z / voxel_norm.y * delta_sides.y in the y direction;
    // IF voxel_norm.x = 0.0, z should not increase in the x direction;
    // IF voxel_norm.y = 0.0, z should not increase in the y direction;
    // we assume that ¬(voxel_norm.z = 0).
    //
    // Now observe that we can rephrase this as saying, given a desired change in z (to get to the next integer), how far must
    // we travel along x and y?
    //
    // TODO: Handle negative numbers.
    // vec3 delta_sides = mix(-fract(f_pos), 1.0 - fract(f_pos), lessThan(sides, vec3(0.0)));
    vec3 delta_sides = mix(fract(f_pos) - 1.0, fract(f_pos), lessThan(sides, vec3(0.0)));
    /* vec3 delta_sides =
        mix(
                mix(-fract(f_pos), 1.0 - fract(f_pos), lessThan(sides, vec3(0.0))),
                mix(-(f_pos - ceil(f_pos)), 1.0 - (f_pos - ceil(f_pos)), lessThan(sides, vec3(0.0))),
                lessThan(f_pos, vec3(0.0))
        ); */
    /* vec3 delta_sides =
        mix(
                mix(1.0 - fract(f_pos), -fract(f_pos), lessThan(sides, vec3(0.0))),
                mix(1.0 - (f_pos - ceil(f_pos)), -(f_pos - ceil(f_pos)), lessThan(sides, vec3(0.0))),
                lessThan(f_pos, vec3(0.0))
        ); */
    // vec3 delta_sides = mix(1.0 - fract(f_pos), -fract(f_pos), lessThan(sides, vec3(0.0)));
    // vec3 delta_sides = 1.0 + sides * fract(-sides * f_pos);
    // delta_sides = -sign(delta_sides) * (1.0 - abs(delta_sides));
    // Three faces: xy, xz, and yz.
    // TODO: Handle zero slopes (for xz and yz).
    vec2 corner_xy = min(abs(f_norm.xy / f_norm.z * delta_sides.z), 1.0);
    vec2 corner_yz = min(abs(f_norm.yz / f_norm.x * delta_sides.x), 1.0);
    vec2 corner_xz = min(abs(f_norm.xz / f_norm.y * delta_sides.y), 1.0);
    // vec3 corner_delta = vec3(voxel_norm.xy / voxel_norm.z * delta_sides.z, delta_sides.z);
    // Now we just compute an (upper bounded) distance to the corner in each direction.
    // vec3 corner_distance = min(abs(corner_delta), 1.0);
    // Now, if both sides hit something, lerp to 0.25.  If one side hits something, lerp to 0.75.  And if no sides hit something,
    // lerp to 1.0 (TODO: incorporate the corner properly).
    // Bilinear interpolation on each plane:
    float ao_xy = dot(vec2(corner_xy.x, 1.0 - corner_xy.x), mat2(vec2(corner_xy.x < 1.00 ? corner_xy.y < 1.00 ? 0.25 : 0.5 : corner_xy.y < 1.00 ? 0.5 : 0.75, corner_xy.x < 1.00 ? 0.75 : 1.00), vec2(corner_xy.y < 1.00 ? 0.75 : 1.0, 1.0)) * vec2(corner_xy.y, 1.0 - corner_xy.y));
    float ao_yz = dot(vec2(corner_yz.x, 1.0 - corner_yz.x), mat2(vec2(corner_yz.x < 1.00 ? corner_yz.y < 1.00 ? 0.25 : 0.5 : corner_yz.y < 1.00 ? 0.5 : 0.75, corner_yz.x < 1.00 ? 0.75 : 1.00), vec2(corner_yz.y < 1.00 ? 0.75 : 1.0, 1.0)) * vec2(corner_yz.y, 1.0 - corner_yz.y));
    float ao_xz = dot(vec2(corner_xz.x, 1.0 - corner_xz.x), mat2(vec2(corner_xz.x < 1.00 ? corner_xz.y < 1.00 ? 0.25 : 0.5 : corner_xz.y < 1.00 ? 0.5 : 0.75, corner_xz.x < 1.00 ? 0.75 : 1.00), vec2(corner_xz.y < 1.00 ? 0.75 : 1.0, 1.0)) * vec2(corner_xz.y, 1.0 - corner_xz.y));
    /* float ao_xy = dot(vec2(1.0 - corner_xy.x, corner_xy.x), mat2(vec2(corner_xy.x < 1.00 ? corner_xy.y < 1.00 ? 0.25 : 0.5 : corner_xy.y < 1.00 ? 0.5 : 0.75, corner_xy.x < 1.00 ? 0.75 : 1.00), vec2(corner_xy.y < 1.00 ? 0.75 : 1.0, 1.0)) * vec2(1.0 - corner_xy.y, corner_xy.y));
    float ao_yz = dot(vec2(1.0 - corner_yz.x, corner_yz.x), mat2(vec2(corner_yz.x < 1.00 ? corner_yz.y < 1.00 ? 0.25 : 0.5 : corner_yz.y < 1.00 ? 0.5 : 0.75, corner_yz.x < 1.00 ? 0.75 : 1.00), vec2(corner_yz.y < 1.00 ? 0.75 : 1.0, 1.0)) * vec2(1.0 - corner_yz.y, corner_yz.y));
    float ao_xz = dot(vec2(1.0 - corner_xz.x, corner_xz.x), mat2(vec2(corner_xz.x < 1.00 ? corner_xz.y < 1.00 ? 0.25 : 0.5 : corner_xz.y < 1.00 ? 0.5 : 0.75, corner_xz.x < 1.00 ? 0.75 : 1.00), vec2(corner_xz.y < 1.00 ? 0.75 : 1.0, 1.0)) * vec2(1.0 - corner_xz.y, corner_xz.y)); */
    // Now, if both sides hit something, lerp to 0.0.  If one side hits something, lerp to 0.4.  And if no sides hit something,
    // lerp to 1.0.
    // Bilinear interpolation on each plane:
    // float ao_xy = dot(vec2(1.0 - corner_xy.x, corner_xy.x), mat2(vec2(corner_xy.x < 1.00 ? corner_xy.y < 1.00 ? 0.0 : 0.25 : corner_xy.y < 1.00 ? 0.25 : 1.0, corner_xy.x < 1.00 ? 0.25 : 1.0), vec2(corner_xy.y < 1.00 ? 0.25 : 1.0, 1.0)) * vec2(1.0 - corner_xy.y, corner_xy.y));
    // float ao_yz = dot(vec2(1.0 - corner_yz.x, corner_yz.x), mat2(vec2(corner_yz.x < 1.00 ? corner_yz.y < 1.00 ? 0.0 : 0.25 : corner_yz.y < 1.00 ? 0.25 : 1.0, corner_yz.x < 1.00 ? 0.25 : 1.0), vec2(corner_yz.y < 1.00 ? 0.25 : 1.0, 1.0)) * vec2(1.0 - corner_yz.y, corner_yz.y));
    // float ao_xz = dot(vec2(1.0 - corner_xz.x, corner_xz.x), mat2(vec2(corner_xz.x < 1.00 ? corner_xz.y < 1.00 ? 0.0 : 0.25 : corner_xz.y < 1.00 ? 0.25 : 1.0, corner_xz.x < 1.00 ? 0.25 : 1.0), vec2(corner_xz.y < 1.00 ? 0.25 : 1.0, 1.0)) * vec2(1.0 - corner_xz.y, corner_xz.y));
    // Now, multiply each component by the face "share" which is just the absolute value of its normal for that plane...
    // vec3 f_ao_vec = mix(abs(vec3(ao_yz, ao_xz, ao_xy)), vec3(1.0), bvec3(f_norm.yz == vec2(0.0), f_norm.xz == vec2(0.0), f_norm.xy == vec2(0.0)));
    // vec3 f_ao_vec = mix(abs(vec3(ao_yz, ao_xz, ao_xy)), vec3(1.0), bvec3(length(f_norm.yz) <= 0.0, length(f_norm.xz) <= 0.0, length(f_norm.xy) <= 0.0));
    // vec3 f_ao_vec = mix(abs(vec3(ao_yz, ao_xz, ao_xy)), vec3(1.0), bvec3(abs(f_norm.x) <= 0.0, abs(f_norm.y) <= 0.0, abs(f_norm.z) <= 0.0));
    vec3 f_ao_vec = mix(/*abs(voxel_norm)*/vec3(1.0, 1.0, 1.0), /*abs(voxel_norm) * */vec3(ao_yz, ao_xz, ao_xy), /*abs(voxel_norm)*/vec3(length(f_norm.yz), length(f_norm.xz), length(f_norm.xy))/*vec3(1.0)*//*sign(max(view_dir * sides, 0.0))*/);
    float f_orig_len = length(cam_pos.xyz - f_pos);
    vec3 f_orig_view_dir = normalize(cam_pos.xyz - f_pos);
    // f_ao_vec *= sign(max(f_orig_view_dir * sides, 0.0));
    // Projecting view onto face:
    // bool IntersectRayPlane(vec3 rayOrigin, vec3 rayDirection, vec3 posOnPlane, vec3 planeNormal, inout vec3 intersectionPoint)
    // {
    //   float rDotn = dot(rayDirection, planeNormal);
    //
    //   //parallel to plane or pointing away from plane?
    //   if (rDotn < 0.0000001 )
    //     return false;
    //
    //   float s = dot(planeNormal, (posOnPlane - rayOrigin)) / rDotn;
    //
    //   intersectionPoint = rayOrigin + s * rayDirection;
    //
    //   return true;
    // }

    bvec3 hit_yz_xz_xy;
    vec3 dist_yz_xz_xy;
    /* {
        // vec3 rDotn = -f_orig_view_dir * -sides;
        // vec3 rDotn = f_orig_view_dir * sides;
        // hit_yz_xz_xy = greaterThanEqual(rDotn, vec3(0.000001));
        // vec3 s = -sides * (f_pos + delta_sides - cam_pos.xyz) / rDotn;
        // dist_yz_xz_xy = abs(s * -f_orig_view_dir);
        // vec3 s = -sides * (f_pos + delta_sides - cam_pos.xyz) / (-f_orig_view_dir * -sides);
        // vec3 s = (f_pos + delta_sides - cam_pos.xyz) / -f_orig_view_dir;
        // dist_yz_xz_xy = abs(s);
        hit_yz_xz_xy = greaterThanEqual(f_orig_view_dir * sides, vec3(0.000001));
        dist_yz_xz_xy = abs((f_pos + delta_sides - cam_pos.xyz) / -f_orig_view_dir);
    } */
    {
        // vec3 rDotn = -f_orig_view_dir * -sides;
        // vec3 rDotn = f_orig_view_dir * sides;
        // hit_yz_xz_xy = greaterThanEqual(rDotn, vec3(0.000001));
        // vec3 s = -sides * (f_pos + delta_sides - cam_pos.xyz) / rDotn;
        // dist_yz_xz_xy = abs(s * -f_orig_view_dir);
        // vec3 s = -sides * (f_pos + delta_sides - cam_pos.xyz) / (-f_orig_view_dir * -sides);
        // vec3 s = (f_pos + delta_sides - cam_pos.xyz) / -f_orig_view_dir;
        // dist_yz_xz_xy = abs(s);
        hit_yz_xz_xy = greaterThanEqual(f_orig_view_dir * sides, vec3(0.000001));
        dist_yz_xz_xy = abs((f_pos + delta_sides - cam_pos.xyz) / f_orig_view_dir);
    }

    // vec3 xy_point = f_pos, xz_point = f_pos, yz_point = f_pos;
    // bool hit_xy = (/*ao_yz < 1.0 || ao_xz < 1.0*//*min(f_ao_vec.x, f_ao_vec.y)*//*f_ao_vec.z < 1.0*/true/*min(corner_xz.y, corner_yz.y) < 1.0*//*min(corner_xy.x, corner_xy.y) < 1.0*/) && IntersectRayPlane(cam_pos.xyz, -f_orig_view_dir, vec3(f_pos.x, f_pos.y, f_pos.z + delta_sides.z/* - sides.z*/), vec3(0.0, 0.0, -sides.z), xy_point);
    // bool hit_xz = (/*ao_xy < 1.0 || ao_yz < 1.0*//*min(f_ao_vec.x, f_ao_vec.z)*//*f_ao_vec.y < 1.0*/true/*min(corner_xy.y, corner_yz.x) < 1.0*//*min(corner_xz.x, corner_xz.y) < 1.0*/) && IntersectRayPlane(cam_pos.xyz, -f_orig_view_dir, vec3(f_pos.x, f_pos.y + delta_sides.y/* - sides.y*/, f_pos.z), vec3(0.0, -sides.y, 0.0), xz_point);
    // bool hit_yz = (/*ao_xy < 1.0 || ao_xz < 1.0*//*min(f_ao_vec.y, f_ao_vec.z) < 1.0*//*f_ao_vec.x < 1.0*/true/*true*//*min(corner_xy.x, corner_xz.x) < 1.0*//*min(corner_yz.x, corner_yz.y) < 1.0*/) && IntersectRayPlane(cam_pos.xyz, -f_orig_view_dir, vec3(f_pos.x + delta_sides.x/* - sides.x*/, f_pos.y, f_pos.z), vec3(-sides.x, 0.0, 0.0), yz_point);
    // float xy_dist = distance(cam_pos.xyz, xy_point), xz_dist = distance(cam_pos.xyz, xz_point), yz_dist = distance(cam_pos.xyz, yz_point);
    bool hit_xy = hit_yz_xz_xy.z, hit_xz = hit_yz_xz_xy.y, hit_yz = hit_yz_xz_xy.x;
    float xy_dist = dist_yz_xz_xy.z, xz_dist = dist_yz_xz_xy.y, yz_dist = dist_yz_xz_xy.x;
    // hit_xy = hit_xy && distance(f_pos.xy + delta_sides.xy, xy_point.xy) <= 1.0;
    // hit_xz = hit_xz && distance(f_pos.xz + delta_sides.xz, xz_point.xz) <= 1.0;
    // hit_yz = hit_yz && distance(f_pos.yz + delta_sides.yz, yz_point.yz) <= 1.0;
    vec3 voxel_norm =
        hit_yz ?
            hit_xz ?
                yz_dist < xz_dist ?
                    hit_xy ? yz_dist < xy_dist ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
                    hit_xy ? xz_dist < xy_dist ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
                hit_xy ? yz_dist < xy_dist ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
            hit_xz ?
                hit_xy ? xz_dist < xy_dist ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
                hit_xy ? vec3(0.0, 0.0, sides.z) : vec3(0.0, 0.0, 0.0);
    // vec3 f_ao_view = max(vec3(dot(f_orig_view_dir.yz, sides.yz), dot(f_orig_view_dir.xz, sides.xz), dot(f_orig_view_dir.xy, sides.xy)), 0.0);
    // delta_sides *= sqrt(1.0 - f_ao_view * f_ao_view);
    // delta_sides *= 1.0 - mix(view_dir / f_ao_view, vec3(0.0), equal(f_ao_view, vec3(0.0)));// sqrt(1.0 - f_ao_view * f_ao_view);
    // delta_sides *= 1.0 - /*sign*/(max(vec3(dot(f_orig_view_dir.yz, sides.yz), dot(f_orig_view_dir.xz, sides.xz), dot(f_orig_view_dir.xy, sides.xy)), 0.0));
    // f_ao = length(f_ao_vec);
    // f_ao = dot(f_ao_vec, vec3(1.0)) / 3.0;
    // f_ao = 1.0 / sqrt(3.0) * sqrt(dot(f_ao_vec, vec3(1.0)));
    // f_ao = pow(f_ao_vec.x * f_ao_vec.y * f_ao_vec.z * 3.0, 1.0 / 2.0); // 1.0 / sqrt(3.0) * sqrt(dot(f_ao_vec, vec3(1.0)));
    // f_ao = pow(f_ao_vec.x * f_ao_vec.y * f_ao_vec.z, 1.0 / 3.0); // 1.0 / sqrt(3.0) * sqrt(dot(f_ao_vec, vec3(1.0)));
    // f_ao = f_ao_vec.x * f_ao_vec.y * f_ao_vec.z + (1.0 - f_ao_vec.x) * (1.0 - f_ao_vec.y) * (1.0 - f_ao_vec.z);
    // f_ao = sqrt((f_ao_vec.x + f_ao_vec.y + f_ao_vec.z) / 3.0); // 1.0 / sqrt(3.0) * sqrt(dot(f_ao_vec, vec3(1.0)));
    // f_ao = sqrt(dot(f_ao_vec, abs(voxel_norm)));
    // f_ao = 3.0 / (1.0 / f_ao_vec.x + 1.0 / f_ao_vec.y + 1.0 / f_ao_vec.z);
    // f_ao = min(ao_yz, min(ao_xz, ao_xy));
    // f_ao = max(f_ao_vec.x, max(f_ao_vec.y, f_ao_vec.z));
    // f_ao = min(f_ao_vec.x, min(f_ao_vec.y, f_ao_vec.z));
    // f_ao = sqrt(dot(f_ao_vec * abs(voxel_norm), sqrt(1.0 - delta_sides * delta_sides)) / 3.0);
    // f_ao = dot(f_ao_vec, sqrt(1.0 - delta_sides * delta_sides));
    // f_ao = dot(f_ao_vec, 1.0 - abs(delta_sides));
    // f_ao =
    //     f_ao_vec.x < 1.0 ?
    //         f_ao_vec.y < 1.0 ?
    //             abs(delta_sides.x) < abs(delta_sides.y) ?
    //                 f_ao_vec.z < 1.0 ? abs(delta_sides.x) < abs(delta_sides.z) ? f_ao_vec.x : f_ao_vec.z : f_ao_vec.x :
    //                 f_ao_vec.z < 1.0 ? abs(delta_sides.y) < abs(delta_sides.z) ? f_ao_vec.y : f_ao_vec.z : f_ao_vec.y :
    //         f_ao_vec.z < 1.0 ? abs(delta_sides.x) < abs(delta_sides.z) ? f_ao_vec.x : f_ao_vec.z : f_ao_vec.x :
    //     f_ao_vec.y < 1.0 ?
    //         f_ao_vec.z < 1.0 ? abs(delta_sides.y) < abs(delta_sides.z) ? f_ao_vec.y : f_ao_vec.z : f_ao_vec.y :
    //         f_ao_vec.z;
    // f_ao = abs(delta_sides.x) < abs(delta_sides.y) ? abs(delta_sides.x) < abs(delta_sides.z) ? f_ao_vec.x : f_ao_vec.z :
    //                                                  abs(delta_sides.y) < abs(delta_sides.z) ? f_ao_vec.y : f_ao_vec.z;
    // f_ao = abs(delta_sides.x) * f_ao_vec.x < abs(delta_sides.y) * f_ao_vec.y ? abs(delta_sides.x) * f_ao_vec.x < abs(delta_sides.z) * f_ao_vec.z ? f_ao_vec.x : f_ao_vec.z :
    //                                                                            abs(delta_sides.y) * f_ao_vec.y < abs(delta_sides.z) * f_ao_vec.z ? f_ao_vec.y : f_ao_vec.z;
    // f_ao = dot(abs(voxel_norm), abs(voxel_norm) * f_ao_vec)/* / 3.0*/;
    // f_ao = sqrt(dot(abs(voxel_norm), f_ao_vec) / 3.0);
    // f_ao = /*abs(sides)*/max(sign(1.0 + f_orig_view_dir * sides), 0.0) * f_ao);
    // f_ao = mix(f_ao, 1.0, dist_lerp);

    // vec3 voxel_norm = f_norm;
    // vec3 voxel_norm =
    //      f_ao_vec.x < 1.0 ?
    //          f_ao_vec.y < 1.0 ?
    //              abs(delta_sides.x) < abs(delta_sides.y) ?
    //                  f_ao_vec.z < 1.0 ? abs(delta_sides.x) < abs(delta_sides.z) ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
    //                  f_ao_vec.z < 1.0 ? abs(delta_sides.y) < abs(delta_sides.z) ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
    //          f_ao_vec.z < 1.0 ? abs(delta_sides.x) < abs(delta_sides.z) ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
    //      f_ao_vec.y < 1.0 ?
    //          f_ao_vec.z < 1.0 ? abs(delta_sides.y) < abs(delta_sides.z) ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
    //          f_ao_vec.z < 1.0 ? vec3(0.0, 0.0, sides.z) : vec3(0.0, 0.0, 0.0);

    // vec3 voxel_norm =
    //      /*f_ao_vec.x < 1.0*/true ?
    //          /*f_ao_vec.y < 1.0*/true ?
    //              abs(delta_sides.x) < abs(delta_sides.y) ?
    //                  /*f_ao_vec.z < 1.0 */true ? abs(delta_sides.x) < abs(delta_sides.z) ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
    //                  /*f_ao_vec.z < 1.0*/true ? abs(delta_sides.y) < abs(delta_sides.z) ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
    //          /*f_ao_vec.z < 1.0*/true ? abs(delta_sides.x) < abs(delta_sides.z) ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
    //      /*f_ao_vec.y < 1.0*/true ?
    //          /*f_ao_vec.z < 1.0*/true ? abs(delta_sides.y) < abs(delta_sides.z) ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
    //          vec3(0.0, 0.0, sides.z);
    /* vec3 voxel_norm =
         f_ao_vec.x < 1.0 ?
             f_ao_vec.y < 1.0 ?
                 abs(delta_sides.x) < abs(delta_sides.y) ?
                     f_ao_vec.z < 1.0 ? abs(delta_sides.x) < abs(delta_sides.z) ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
                     f_ao_vec.z < 1.0 ? abs(delta_sides.y) < abs(delta_sides.z) ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
             f_ao_vec.z < 1.0 ? abs(delta_sides.x) < abs(delta_sides.z) ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
         f_ao_vec.y < 1.0 ?
             f_ao_vec.z < 1.0 ? abs(delta_sides.y) < abs(delta_sides.z) ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
             f_ao_vec.z < 1.0 ? vec3(0.0, 0.0, sides.z) : vec3(0.0, 0.0, 0.0); */
    // vec3 voxel_norm =
    //      f_ao_vec.x < 1.0 ?
    //          f_ao_vec.y < 1.0 ?
    //              abs(delta_sides.x) * f_ao_vec.x < abs(delta_sides.y) * f_ao_vec.y ?
    //                  f_ao_vec.z < 1.0 ? abs(delta_sides.x) * f_ao_vec.x < abs(delta_sides.z) * f_ao_vec.z ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
    //                  f_ao_vec.z < 1.0 ? abs(delta_sides.y) * f_ao_vec.y < abs(delta_sides.z) * f_ao_vec.z ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
    //          f_ao_vec.z < 1.0 ? abs(delta_sides.x) * f_ao_vec.x < abs(delta_sides.z) * f_ao_vec.z ? vec3(sides.x, 0.0, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(sides.x, 0.0, 0.0) :
    //      f_ao_vec.y < 1.0 ?
    //          f_ao_vec.z < 1.0 ? abs(delta_sides.y) * f_ao_vec.y < abs(delta_sides.z) * f_ao_vec.z ? vec3(0.0, sides.y, 0.0) : vec3(0.0, 0.0, sides.z) : vec3(0.0, sides.y, 0.0) :
    //          f_ao_vec.z < 1.0 ? vec3(0.0, 0.0, sides.z) : vec3(0.0, 0.0, 0.0);
    // vec3 voxel_norm = vec3(0.0);
    // voxel_norm = mix(voxel_norm, f_norm, dist_lerp);

    /* vec3 sun_dir = get_sun_dir(time_of_day.x);
    vec3 moon_dir = get_moon_dir(time_of_day.x); */
    // voxel_norm = vec3(0.0);

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP || FLUID_MODE == FLUID_MODE_SHINY)
    float shadow_alt = /*f_pos.z;*/alt_at(f_pos.xy);//max(alt_at(f_pos.xy), f_pos.z);
    // float shadow_alt = f_pos.z;
#elif (SHADOW_MODE == SHADOW_MODE_NONE || FLUID_MODE == FLUID_MODE_CHEAP)
    float shadow_alt = f_pos.z;
#endif

#if (SHADOW_MODE == SHADOW_MODE_CHEAP || SHADOW_MODE == SHADOW_MODE_MAP)
    vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
    float sun_shade_frac = horizon_at2(f_shadow, shadow_alt, f_pos, sun_dir);
    // float sun_shade_frac = 1.0;
#elif (SHADOW_MODE == SHADOW_MODE_NONE)
    float sun_shade_frac = 1.0;//horizon_at2(f_shadow, shadow_alt, f_pos, sun_dir);
#endif
    float moon_shade_frac = 1.0;//horizon_at2(f_shadow, shadow_alt, f_pos, moon_dir);

    f_pos.xyz += abs(voxel_norm) * delta_sides;
    voxel_norm = voxel_norm == vec3(0.0) ? f_norm : voxel_norm;

    vec3 hash_pos = f_pos + focus_off.xyz;
    f_col = /*srgb_to_linear*/(f_col + hash(vec4(floor(hash_pos * 3.0 - voxel_norm * 0.5), 0)) * 0.01/* - 0.01*/); // Small-scale noise

    // f_ao = 1.0;
    // f_ao = dot(f_ao_vec, sqrt(1.0 - delta_sides * delta_sides));

    f_ao = dot(f_ao_vec, abs(voxel_norm));
    // f_ao = sqrt(dot(f_ao_vec * abs(voxel_norm), sqrt(1.0 - delta_sides * delta_sides)) / 3.0);

    // vec3 ao_pos2 = min(fract(f_pos), 1.0 - fract(f_pos));
    // f_ao = sqrt(dot(ao_pos2, ao_pos2));
    // // f_ao = dot(abs(voxel_norm), f_ao_vec);
    // // voxel_norm = f_norm;

    // Note: because voxels, we reduce the normal for reflections to just its z component, dpendng on distance to camera.
    // Idea: the closer we are to facing top-down, the more the norm should tend towards up-z.
    // vec3 l_norm; // = vec3(0.0, 0.0, 1.0);
    // vec3 l_norm = normalize(vec3(f_norm.x / max(abs(f_norm.x), 0.001), f_norm.y / max(abs(f_norm.y), 0.001), f_norm.z / max(abs(f_norm.z), 0.001)));
    // vec3 l_factor = 1.0 / (1.0 + max(abs(/*f_pos - cam_pos.xyz*//*-vec3(vert_pos4) / vert_pos4.w*/vec3(f_pos.xy, 0.0) - vec3(/*cam_pos*/focus_pos.xy, cam_to_frag)) - vec3(view_distance.x, view_distance.x, 0.0), 0.0) / vec3(32.0 * 2.0, 32.0 * 2.0, 1.0));
    // l_factor.z =
    // vec4 focus_pos4 = view_mat * vec4(focus_pos.xyz, 1.0);
    // vec3 focus_dir = normalize(-vec3(focus_pos4) / focus_pos4.w);

    // float l_factor = 1.0 - pow(clamp(0.5 + 0.5 * dot(/*-view_dir*/-cam_to_frag, l_norm), 0.0, 1.0), 2.0);//1.0 / (1.0 + 0.5 * pow(max(distance(/*focus_pos.xy*/vec3(focus_pos.xy, /*vert_pos4.z / vert_pos4.w*/f_pos.z), vec3(f_pos.xy, f_pos.z))/* - view_distance.x*/ - 32.0, 0.0) / (32.0 * 1.0), /*0.5*/1.0));
    // l_factor = 1.0;
    // l_norm = normalize(mix(l_norm, f_norm, l_factor));
    // l_norm = f_norm;

    /* l_norm = normalize(vec3(
            mix(l_norm.x, f_norm.x, clamp(pow(f_norm.x * 0.5, 64), 0, 1)),
            mix(-1.0, 1.0, clamp(pow(f_norm.y * 0.5, 64), 0, 1)),
            mix(-1.0, 1.0, clamp(pow(f_norm.z * 0.5, 64), 0, 1))
        )); */
    // f_norm = mix(l_norm, f_norm, min(1.0 / max(cam_to_frag, 0.001), 1.0));
    /* vec3 l_norm = normalize(vec3(
            mix(-1.0, 1.0, clamp(pow(f_norm.x * 0.5, 64), 0, 1)),
            mix(-1.0, 1.0, clamp(pow(f_norm.y * 0.5, 64), 0, 1)),
            mix(-1.0, 1.0, clamp(pow(f_norm.z * 0.5, 64), 0, 1))
        )); */
    vec3 cam_to_frag = normalize(f_pos - cam_pos.xyz);
    vec3 view_dir = -cam_to_frag;
    // vec3 view_dir = normalize(f_pos - cam_pos.xyz);


    // vec3 sun_dir = get_sun_dir(time_of_day.x);
    // vec3 moon_dir = get_moon_dir(time_of_day.x);
    // // float sun_light = get_sun_brightness(sun_dir);
    // // float moon_light = get_moon_brightness(moon_dir);
    // // float my_alt = f_pos.z;//alt_at_real(f_pos.xy);
    // // vec3 f_norm = my_norm;
    // // vec4 f_shadow = textureBicubic(t_horizon, pos_to_tex(f_pos.xy));
    // // float shadow_alt = /*f_pos.z;*/alt_at(f_pos.xy);//max(alt_at(f_pos.xy), f_pos.z);
    // // float my_alt = alt_at(f_pos.xy);
    // float sun_shade_frac = horizon_at2(f_shadow, shadow_alt, f_pos, sun_dir);
    // float moon_shade_frac = horizon_at2(f_shadow, shadow_alt, f_pos, moon_dir);
    // // float sun_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, sun_dir);
    // // float moon_shade_frac = horizon_at(/*f_shadow, f_pos.z, */f_pos, moon_dir);
    // // Globbal illumination "estimate" used to light the faces of voxels which are parallel to the sun or moon (which is a very common occurrence).
    // // Will be attenuated by k_d, which is assumed to carry any additional ambient occlusion information (e.g. about shadowing).
    // // float ambient_sides = clamp(mix(0.5, 0.0, abs(dot(-f_norm, sun_dir)) * 10000.0), 0.0, 0.5);
    // // NOTE: current assumption is that moon and sun shouldn't be out at the sae time.
    // // This assumption is (or can at least easily be) wrong, but if we pretend it's true we avoids having to explicitly pass in a separate shadow
    // // for the sun and moon (since they have different brightnesses / colors so the shadows shouldn't attenuate equally).
    // // float shade_frac = sun_shade_frac + moon_shade_frac;
    // // float brightness_denominator = (ambient_sides + vec3(SUN_AMBIANCE * sun_light + moon_light);

    // DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, light_pos);
    DirectionalLight sun_info = get_sun_info(sun_dir, sun_shade_frac, /*sun_pos*/f_pos);
    DirectionalLight moon_info = get_moon_info(moon_dir, moon_shade_frac/*, light_pos*/);

    float alpha = 1.0;//0.1;//0.2;///1.0;//sqrt(2.0);
    const float n2 = 1.5;
    const float R_s2s0 = pow((1.0 - n2) / (1.0 + n2), 2);
    const float R_s1s0 = pow((1.3325 - n2) / (1.3325 + n2), 2);
    const float R_s2s1 = pow((1.0 - 1.3325) / (1.0 + 1.3325), 2);
    const float R_s1s2 = pow((1.3325 - 1.0) / (1.3325 + 1.0), 2);
    float cam_alt = alt_at(cam_pos.xy);
    float fluid_alt = medium.x == 1u ? max(cam_alt + 1, floor(shadow_alt)) : view_distance.w;
    float R_s = (f_pos.z < my_alt) ? mix(R_s2s1 * R_s1s0, R_s1s0, medium.x) : mix(R_s2s0, R_s1s2 * R_s2s0, medium.x);

    vec3 emitted_light, reflected_light;

    vec3 mu = medium.x == 1u/* && f_pos.z <= fluid_alt*/ ? MU_WATER : vec3(0.0);
    // NOTE: Default intersection point is camera position, meaning if we fail to intersect we assume the whole camera is in water.
    vec3 cam_attenuation = compute_attenuation_point(cam_pos.xyz, view_dir, mu, fluid_alt, /*cam_pos.z <= fluid_alt ? cam_pos.xyz : f_pos*/f_pos);
    // Use f_norm here for better shadows.
    // vec3 light_frac = light_reflection_factor(f_norm/*l_norm*/, view_dir, vec3(0, 0, -1.0), vec3(1.0), vec3(/*1.0*/R_s), alpha);

    // vec3 light, diffuse_light, ambient_light;
    // get_sun_diffuse(f_norm, time_of_day.x, cam_to_frag, (0.25 * shade_frac + 0.25 * light_frac) * f_col, 0.5 * shade_frac * f_col, 0.5 * shade_frac * /*vec3(1.0)*/f_col, 2.0, emitted_light, reflected_light);
    float max_light = 0.0;
    max_light += get_sun_diffuse2(sun_info, moon_info, voxel_norm/*l_norm*/, view_dir, f_pos, vec3(0.0), cam_attenuation, fluid_alt, vec3(1.0)/* * (0.5 * light_frac + vec3(0.5 * shade_frac))*/, vec3(1.0), /*0.5 * shade_frac * *//*vec3(1.0)*//*f_col*/vec3(R_s), alpha, voxel_norm, dist_lerp/*max(distance(focus_pos.xy, f_pos.xyz) - view_distance.x, 0.0) / 1000 < 1.0*/, emitted_light, reflected_light);
    // emitted_light = vec3(1.0);
    // emitted_light *= max(shade_frac, MIN_SHADOW);
    // reflected_light *= shade_frac;
    // max_light *= shade_frac;
    // reflected_light = vec3(0.0);

    // dot(diffuse_factor, /*R_r * */vec4(abs(norm) * (1.0 - dist), dist))

    // corner_xy = mix(all(lessThan(corner_xy, 1.0)) ? vec2(0.0) : 0.4 * (), 1.0
    //
    // TODO: Handle similar logic for z.

    // So we repeat this for all three sides to find the "next" position on each side.
    // vec3 delta_sides = 1.0 + sides * fract(-sides * f_pos);
    // Now, we
    // Now, all we have to do is find out whether (again, assuming f_pos is positive) next_sides represents a new integer.
    // We currently just treat this as "new floor != old floor".

    // So to find the position at the nearest voxel, we just subtract voxel_norm * fract(sides * ) from f_pos.z.
    // Then to find out whether we meet a new "block" in 1 voxel, we just
    // on the "other" side can be found (according to my temporary theory) as the cross product
    // vec3 norm = normalize(cross(
    //     vec3(/*2.0 * SAMPLE_W*/square.z - square.x, 0.0, altx1 - altx0),
    //     vec3(0.0, /*2.0 * SAMPLE_W*/square.w - square.y, alty1 - alty0)
    // ));
    // vec3 norm = normalize(vec3(
    //     (altx0 - altx1) / (square.z - square.x),
    //     (alty0 - alty1) / (square.w - square.y),
    //     1.0
    //     //(abs(square.w - square.y) + abs(square.z - square.x)) / (slope + 0.00001) // Avoid NaN
    // ));
    //
    // If a side coordinate is 0, then it counts as no AO;
    // otherwise, it counts as fractional AO.  So what we need is to know whether the fractional AO to the next block in that direction pushes us to a new integer.
    //
    // vec3 ao_pos_z = floor(f_pos + f_norm);
    // vec3 ao_pos_z = corner_distance;
    // vec3 ao_pos = 0.5 - clamp(min(fract(abs(f_pos)), 1.0 - fract(abs(f_pos))), 0.0, 0.5);
    //
    // f_ao = /*sqrt*/1.0 - 2.0 * sqrt(dot(ao_pos, ao_pos) / 2.0);
    // f_ao = /*sqrt*/1.0 - (dot(ao_pos, ao_pos)/* / 2.0*/);
    // f_ao = /*sqrt*/1.0 - 2.0 * (dot(ao_pos, ao_pos)/* / 2.0*/);
    // f_ao = /*sqrt*/1.0 - 2.0 * sqrt(dot(ao_pos, ao_pos) / 2.0);
    float ao = f_ao;// /*pow(f_ao, 0.5)*/f_ao * 0.9 + 0.1;
    emitted_light *= ao;
    reflected_light *= ao;

    // emitted_light += 0.5 * vec3(SUN_AMBIANCE * sun_shade_frac * sun_light + moon_shade_frac * moon_light) * f_col * (ambient_sides + 1.0);

    // Ambient lighting attempt: vertical light.
    // reflected_light += /*0.0125*/0.15 * 0.25 * _col * light_reflection_factor(f_norm, cam_to_frag, vec3(0, 0, -1.0), 0.5 * f_col, 0.5 * f_col, 2.0);
    // emitted_light += /*0.0125*/0.25 * f_col * ;
    // vec3 light, diffuse_light, ambient_light;
    // get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
    // vec3 surf_color = illuminate(f_col, light, diffuse_light, ambient_light);
    // f_col = f_col + (hash(vec4(floor(vec3(focus_pos.xy + splay(v_pos_orig), f_pos.z)) * 3.0 - round(f_norm) * 0.5, 0)) - 0.5) * 0.05; // Small-scale noise
    vec3 surf_color = /*illuminate(emitted_light, reflected_light)*/illuminate(max_light, view_dir, f_col * emitted_light, f_col * reflected_light);

    float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);

#if (CLOUD_MODE == CLOUD_MODE_REGULAR)
    vec4 clouds;
    vec3 fog_color = get_sky_color(cam_to_frag/*view_dir*/, time_of_day.x, cam_pos.xyz, f_pos, 1.0, /*true*/false, clouds);
    vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);
#elif (CLOUD_MODE == CLOUD_MODE_NONE)
    vec3 color = surf_color;
#endif

    // float mist_factor = max(1 - (f_pos.z + (texture(t_noise, f_pos.xy * 0.0005 + time_of_day.x * 0.0003).x - 0.5) * 128.0) / 400.0, 0.0);
    // //float mist_factor = f_norm.z * 2.0;
    // color = mix(color, vec3(1.0) * /*diffuse_light*/reflected_light, clamp(mist_factor * 0.00005 * distance(f_pos.xy, focus_pos.xy), 0, 0.3));
    // color = surf_color;

    tgt_color = vec4(color, 1.0);
}
