use core::{iter, mem};
use hashbrown::HashMap;
use num::traits::Float;
pub use vek::{geom::repr_simd::*, mat::repr_simd::column_major::Mat4, ops::*, vec::repr_simd::*};
//pub use vek::{geom::repr_c::*, mat::repr_c::column_major::Mat4, ops::*,
// vec::repr_c::*};

pub fn aabb_to_points<T: Float>(bounds: Aabb<T>) -> [Vec3<T>; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
    ]
}

/// Each Vec4 <a, b, c, -d> should be interpreted as reprenting plane
/// equation
///
/// a(x - x0) + b(y - y0) + c(z - z0) = 0, i.e.
/// ax + by + cz - (a * x0 + b * y0 + c * z0) = 0, i.e.
/// ax + by + cz = (a * x0 + b * y0 + c * z0), i.e.
/// (letting d = a * x0 + b * y0 + c * z0)
/// ax + by + cz = d
///
/// where d is the distance of the plane from the origin.
pub fn aabb_to_planes<T: Float>(bounds: Aabb<T>) -> [Vec4<T>; 6] {
    let zero = T::zero();
    let one = T::one();
    let bounds = bounds.map(|e| e.abs());
    [
        // bottom
        Vec4::new(zero, -one, zero, -bounds.min.y),
        // top
        Vec4::new(zero, one, zero, -bounds.max.y),
        // left
        Vec4::new(-one, zero, zero, -bounds.min.x),
        // right
        Vec4::new(one, zero, zero, -bounds.max.x),
        // near
        Vec4::new(zero, zero, -one, -bounds.min.z),
        // far
        Vec4::new(zero, zero, one, -bounds.max.z),
    ]
}

pub fn mat_mul_points<T: Float + MulAdd<T, T, Output = T>>(
    mat: Mat4<T>,
    pts: &mut [Vec3<T>],
    mut do_p: impl FnMut(Vec4<T>) -> Vec3<T>,
) {
    pts.iter_mut().for_each(|p| {
        *p = do_p(mat * Vec4::from_point(*p));
    });
}

/// NOTE: Expects points computed from aabb_to_points.
pub fn calc_view_frust_object<T: Float>(pts: &[Vec3<T>; 8]) -> Vec<Vec<Vec3<T>>> {
    vec![
        // near (CCW)
        vec![pts[0], pts[1], pts[2], pts[3]],
        // far (CCW)
        vec![pts[7], pts[6], pts[5], pts[4]],
        // left (CCW)
        vec![pts[0], pts[3], pts[7], pts[4]],
        // right (CCW)
        vec![pts[1], pts[5], pts[6], pts[2]],
        // bottom (CCW)
        vec![pts[4], pts[5], pts[1], pts[0]],
        // top (CCW)
        vec![pts[6], pts[7], pts[3], pts[2]],
    ]
}

pub fn calc_view_frustum_world_coord<T: Float + MulAdd<T, T, Output = T>>(
    inv_proj_view: Mat4<T>,
) -> [Vec3<T>; 8] {
    let mut world_pts = aabb_to_points(Aabb {
        min: Vec3::new(-T::one(), -T::one(), T::zero()),
        max: Vec3::one(),
    });
    mat_mul_points(inv_proj_view, &mut world_pts, |p| Vec3::from(p) / p.w);
    world_pts
}

pub fn point_plane_distance<T: Float>(point: Vec3<T>, norm_dist: Vec4<T>) -> T {
    norm_dist.dot(Vec4::from_point(point))
}

pub fn point_before_plane<T: Float>(point: Vec3<T>, plane: Vec4<T>) -> bool {
    point_plane_distance(point, plane) > T::zero()
}

/// Returns true if and only if the final point in the polygon (i.e. the
/// first point added to the new polygon) is outside the clipping plane
/// (this implies that the polygon must be non-degenerate).
pub fn clip_points_by_plane<T: Float + MulAdd<T, T, Output = T> + core::fmt::Debug>(
    points: &mut Vec<Vec3<T>>,
    plane: Vec4<T>,
    intersection_points: &mut Vec<Vec3<T>>,
) -> bool {
    if points.len() < 3 {
        return false;
    }
    // NOTE: Guaranteed to succeed since points.len() > 3.
    let mut current_point = points[points.len() - 1];
    let intersect_plane_edge = |a, b| {
        let diff: Vec3<_> = b - a;
        let t = plane.dot(Vec4::from_direction(diff));
        if t == T::zero() {
            None
        } else {
            let t = -(plane.dot(Vec4::from_point(a)) / t);
            if t < T::zero() || T::one() < t {
                None
            } else {
                Some(diff * t + a)
            }
        }
    };
    let last_is_outside = point_before_plane(current_point, plane);
    let mut is_outside = last_is_outside;
    let mut old_points = Vec::with_capacity((3 * points.len()) / 2);
    mem::swap(&mut old_points, points);
    old_points.into_iter().for_each(|point| {
        let prev_point = mem::replace(&mut current_point, point);
        let before_plane = point_before_plane(current_point, plane);
        let prev_is_outside = mem::replace(&mut is_outside, before_plane);
        if !prev_is_outside {
            // Push previous point.
            points.push(prev_point);
        }
        if prev_is_outside != is_outside {
            if let Some(intersection_point) = intersect_plane_edge(prev_point, current_point) {
                // Push intersection point.
                intersection_points.push(intersection_point);
                points.push(intersection_point);
            }
        }
    });
    last_is_outside
}

fn append_intersection_points<T: Float + core::fmt::Debug>(
    polys: &mut Vec<Vec<Vec3<T>>>,
    intersection_points: Vec<Vec3<T>>,
    tolerance: T,
) {
    // NOTE: We use decoded versions of each polygon, with rounded entries.
    //
    // The line segments in intersection_points are consistently ordered as follows:
    // each segment represents the intersection of the cutting plane with the
    // polygon from which the segment came.  The polygon can thus be split into
    // two parts: the part "inside" the new surface (below the plane), and the
    // part "outside" it (above the plane).  Thus, when oriented
    // with the polygon normal pointing into the camera, and the cutting plane as
    // the x axis, with the "outside" part on top and the "inside" part on the
    // bottom, there is a leftmost point (the point going from outside to
    // inside, counterclockwise) and a rightmost point (the point going from
    // inside to outside, counterclockwise).  Our consistent ordering guarantees
    // that the leftmost point comes before the rightmost point in each line
    // segment.
    //
    // Why does this help us?  To see that, consider the polygon adjacent to the
    // considered polygon which also has the same right intersection point (we
    // know there will be exactly one of these, because we have a solid
    // structure and are only considering polygons that intersect the plane
    // exactly two times; this means that we are ignoring polygons that intersect
    // the plane at just one point, which means the two polygons must share a
    // point, not be coplanar, and both intersect the plane; no theorem here,
    // but I believe there can provably be at most one such instance given that
    // we have at least three polygons with such a line segment).
    //
    // Now, for the adjacent polygon, repeat the above process.  If the intersection
    // point shared by the polygons is on the right in both cases, then we can
    // see that the polygon's normal must be facing in the opposite direction of
    // the original polygon despite being adjacent.  But this
    // should be impossible for a closed object!  The same applies to the leftmost
    // point.
    //
    // What is the practical upshot of all this?  It means that we can consistently
    // hash each line segment by its first point, which we can look up using the
    // second point of a previous line segment.  This will produce a chain of
    // entries terminating in the original segment we looked up.  As an added
    // bonus, by going from leftmost point to leftmost point, we also ensure that
    // we produce a polygon whose face is oriented counterclockwise around its
    // normal; this can be seen by following the right-hand rule (TODO: provide
    // more rigorous proof).
    let tol = tolerance.recip();
    let make_key = move |point: Vec3<T>| {
        // We use floating points rounded to tolerance in order to make our HashMap
        // lookups work. Otherwise we'd have to use a sorted structure, like a
        // btree, which wouldn't be the end of the world but would have
        // theoretically worse complexity.
        //
        // NOTE: Definitely non-ideal that we panic if the rounded value can't fit in an
        // i64...
        //
        // TODO: If necessary, let the caller specify how to hash these keys, since in
        // cases where we know the kind of floating point we're using we can
        // just cast to bits or something.
        point.map(|e| {
            (e * tol)
                .round()
                .to_i64()
                .expect("We don't currently try to handle floats that won't fit in an i64.")
        })
    };
    let mut lines_iter = intersection_points.chunks_exact(2).filter_map(|uv| {
        let u_key = make_key(uv[0]);
        let v = uv[1];
        // NOTE: The reason we need to make sure this doesn't happen is that it's
        // otherwise possible for two points to hash to the same value due to
        // epsilon being too low. Because of the ordering mentioned previously,
        // we know we should *eventually* find a pair of points starting with
        // make_key(u) and ending with a different make_key(v) in such cases, so
        // we just discard all the other pairs (treating them as points rather
        // than lines).
        (u_key != make_key(v)).then_some((u_key, v))
    });

    if let Some((last_key, first)) = lines_iter.next() {
        let lines = lines_iter.collect::<HashMap<_, _>>();
        if lines.len() < 2 {
            // You need at least 3 sides for a polygon
            return;
        }
        // NOTE: Guaranteed to terminate, provided we have no cycles besides the one
        // that touches every point (which should be the case given how these
        // points were generated).
        let poly_iter = iter::successors(Some(first), |&cur| lines.get(&make_key(cur)).copied());
        let poly: Vec<_> = poly_iter.collect();
        // We have to check to make sure we really went through the whole cycle.
        // TODO: Consider adaptively decreasing precision until we can make the cycle
        // happen.
        if poly.last().copied().map(make_key) == Some(last_key) {
            // Push the new polygon onto the object.
            polys.push(poly);
        }
    }
}

pub fn clip_object_by_plane<T: Float + MulAdd<T, T, Output = T> + core::fmt::Debug>(
    polys: &mut Vec<Vec<Vec3<T>>>,
    plane: Vec4<T>,
    tolerance: T,
) {
    let mut intersection_points = Vec::new();
    polys.drain_filter(|points| {
        let len = intersection_points.len();
        let outside_first = clip_points_by_plane(points, plane, &mut intersection_points);
        // Only remember intersections that are not coplanar with this side; i.e. those
        // that have segment length 2.
        if len + 2 != intersection_points.len() {
            intersection_points.truncate(len);
        } else if !outside_first {
            // Order the two intersection points consistently, so that, when considered
            // counterclockwise:
            // - the first point goes from the exterior of the polygon (above the cutting
            //   plane) to its interior.
            // - the second point goes from the interior of the polygon (below the cutting
            //   plane) to its exterior.
            // the second is always going
            //
            // This allows us to uniquely map each line segment to an "owning" point (the
            // one going from outside to inside), which happens to also point
            // the segment in a counterclockwise direction around the new
            // polygon normal composed of all the lines we clipped.
            intersection_points.swap(len, len + 1);
        }
        // Remove polygon if it was clipped away
        points.is_empty()
    });
    // Add a polygon of all intersection points with the plane to close out the
    // object.
    append_intersection_points(polys, intersection_points, tolerance);
}

pub fn clip_object_by_aabb<T: Float + MulAdd<T, T, Output = T> + core::fmt::Debug>(
    polys: &mut Vec<Vec<Vec3<T>>>,
    bounds: Aabb<T>,
    tolerance: T,
) {
    let planes = aabb_to_planes(bounds);
    planes.iter().for_each(|&plane| {
        clip_object_by_plane(polys, plane, tolerance);
    });
}

/// Return value is 'Some(segment)' if line segment intersects the current
/// test plane.  Otherwise 'None' is returned in which case the line
/// segment is entirely clipped.
pub fn clip_test<T: Float + core::fmt::Debug>(p: T, q: T, (u1, u2): (T, T)) -> Option<(T, T)> {
    if p == T::zero() {
        if q >= T::zero() { Some((u1, u2)) } else { None }
    } else {
        let r = q / p;
        if p < T::zero() {
            if r > u2 {
                None
            } else {
                Some((if r > u1 { r } else { u1 }, u2))
            }
        } else if r < u1 {
            None
        } else {
            Some((u1, if r < u2 { r } else { u2 }))
        }
    }
}

pub fn intersection_line_aabb<T: Float + MulAdd<T, T, Output = T> + core::fmt::Debug>(
    p: Vec3<T>,
    dir: Vec3<T>,
    bounds: Aabb<T>,
) -> Option<Vec3<T>> {
    clip_test(-dir.z, p.z - bounds.min.z, (T::zero(), T::infinity()))
        .and_then(|t| clip_test(dir.z, bounds.max.z - p.z, t))
        .and_then(|t| clip_test(-dir.y, p.y - bounds.min.y, t))
        .and_then(|t| clip_test(dir.y, bounds.max.y - p.y, t))
        .and_then(|t| clip_test(-dir.x, p.x - bounds.min.x, t))
        .and_then(|t| clip_test(dir.x, bounds.max.x - p.x, t))
        .and_then(|(t1, t2)| {
            if T::zero() <= t2 {
                Some(dir.mul_add(Vec3::broadcast(t2), p))
            } else if T::zero() <= t1 {
                Some(dir.mul_add(Vec3::broadcast(t1), p))
            } else {
                None
            }
        })
}

pub fn include_object_light_volume<
    T: Float + MulAdd<T, T, Output = T> + core::fmt::Debug,
    I: Iterator<Item = Vec3<T>>,
>(
    obj: I,
    light_dir: Vec3<T>,
    bounds: Aabb<T>,
) -> impl Iterator<Item = Vec3<T>> {
    obj.flat_map(move |pt| iter::once(pt).chain(intersection_line_aabb(pt, -light_dir, bounds)))
}

// NOTE: Currently specialized to skip extending to the end of the light ray,
// since our light ray is already infinite.  Correct code is commented out
// below.
pub fn calc_focused_light_volume_points<T: Float + MulAdd<T, T, Output = T> + core::fmt::Debug>(
    inv_proj_view: Mat4<T>,
    _light_dir: Vec3<T>,
    scene_bounding_box: Aabb<T>,
    tolerance: T,
) -> impl Iterator<Item = Vec3<T>> {
    let world_pts = calc_view_frustum_world_coord(inv_proj_view);
    let mut world_frust_object = calc_view_frust_object(&world_pts);
    clip_object_by_aabb(&mut world_frust_object, scene_bounding_box, tolerance);
    world_frust_object.into_iter().flat_map(|e| e.into_iter())
    /* include_object_light_volume(
        world_frust_object.into_iter().flat_map(|e| e.into_iter()),
        light_dir,
        scene_bounding_box,
    ) */
}

/// Computes an axis aligned bounding box that contains the provided points when
/// transformed into the coordinate space specificed by `mat`.
///
/// "psr" stands for "Potential shadow receivers" since this function is used to
/// get an Aabb containing the potential points (which represent the extents of
/// the volumes of potential things that will be shadowed) that will be
/// shadowed.
///
/// NOTE: Will not yield useful results if pts is empty!
pub fn fit_psr<
    T: Float + MulAdd<T, T, Output = T>,
    I: Iterator<Item = Vec3<T>>,
    F: FnMut(Vec4<T>) -> Vec4<T>,
>(
    mat: Mat4<T>,
    pts: I,
    mut do_p: F,
) -> Aabb<T> {
    let mut min = Vec4::broadcast(T::infinity());
    let mut max = Vec4::broadcast(T::neg_infinity());
    pts.map(|p| do_p(mat * Vec4::<T>::from_point(p)))
        .for_each(|p| {
            min = Vec4::partial_min(min, p);
            max = Vec4::partial_max(max, p);
        });
    Aabb {
        min: min.xyz(),
        max: max.xyz(),
    }
}
