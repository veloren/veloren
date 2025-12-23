use vek::{LineSegment2, LineSegment3, Vec2, Vec3};

// Get closest point between 2 3D line segments https://math.stackexchange.com/a/4289668
pub fn closest_points_3d(n: LineSegment3<f32>, m: LineSegment3<f32>) -> (Vec3<f32>, Vec3<f32>) {
    let p1 = n.start;
    let p2 = n.end;
    let p3 = m.start;
    let p4 = m.end;

    let d1 = p2 - p1;
    let d2 = p4 - p3;
    let d21 = p3 - p1;

    let v22 = d2.dot(d2);
    let v11 = d1.dot(d1);
    let v21 = d2.dot(d1);
    let v21_1 = d21.dot(d1);
    let v21_2 = d21.dot(d2);

    let denom = v21 * v21 - v22 * v11;

    let (s, t) = if denom == 0.0 {
        let s = 0.0;
        let t = (v11 * s - v21_1) / v21;
        (s, t)
    } else {
        let s = (v21_2 * v21 - v22 * v21_1) / denom;
        let t = (-v21_1 * v21 + v11 * v21_2) / denom;
        (s, t)
    };

    let (s, t) = (s.clamp(0.0, 1.0), t.clamp(0.0, 1.0));

    let p_a = p1 + s * d1;
    let p_b = p3 + t * d2;

    (p_a, p_b)
}

/// Line result type
pub enum LineIntersection<T>
where
    T: num_traits::Float + Copy,
{
    /// The intersection is a point.
    Point(Vec2<T>),
    /// The lines are coincident and do not intersect.
    Coincident,
    /// The lines are parallel and do not intersect.
    Parallel,
}

/// Calculate the intersection of two 2D lines.
/// The lines are defined by two line segments, and the intersection may lie
/// outside either segment. I.e., for intersection purposes the lines are
/// considered infinite. This function does not guarantee that the intersection
/// point lies within the bounds of the line segments or that the intersection
/// point lies within some coordinate space (like world size).
///
/// # Arguments
/// * `n` - The first line segment.
/// * `m` - The second line segment.
///
/// # Returns
/// The intersection type.
/// * LineIntersection::Point if there is an intersection.
/// * LineIntersection::Coincident if the lines are coincident and there is no
///   intersection. This means the lines lie on top of each other. They are
///   parallel but have no separation. It could be said that they intersect at
///   infinitely many points.
/// * LineIntersection::Parallel if the lines are parallel and there is no
///   intersection.
pub fn line_intersection_2d<T>(n: LineSegment2<T>, m: LineSegment2<T>) -> LineIntersection<T>
where
    T: num_traits::Float + Copy,
{
    let a = n.end.x - n.start.x;
    let b = -(m.end.x - m.start.x);
    let c = m.start.x - n.start.x;
    let d = n.end.y - n.start.y;
    let e = -(m.end.y - m.start.y);
    let f = m.start.y - n.start.y;

    let num1 = c * e - b * f;
    let num2 = a * f - c * d;
    let denom = a * e - b * d;

    if denom.abs() < T::epsilon() {
        if num1.abs() < T::epsilon() && num2.abs() < T::epsilon() {
            // Lines are coincident.
            return LineIntersection::Coincident;
        }
        // Lines are parallel.
        return LineIntersection::Parallel;
    }

    LineIntersection::Point(Vec2::new(
        n.start.x + (num1 / denom) * a,
        n.start.y + (num1 / denom) * d,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vek::{LineSegment2, Vec2};

    #[macro_export]
    macro_rules! vec_to_line_segment2 {
        ($vec:expr) => {
            LineSegment2 {
                start: Vec2::new($vec[0], $vec[1]),
                end: Vec2::new($vec[2], $vec[3]),
            }
        };
    }

    #[test]
    fn test_intersecting_line() {
        let l1 = [0.0f32, 0.0, 2.0, 2.0];
        let l2 = [0.0f32, 2.0, 2.0, 0.0];
        let n = vec_to_line_segment2!(l1);
        let m = vec_to_line_segment2!(l2);
        match line_intersection_2d(n, m) {
            LineIntersection::Point(p) => {
                assert!((p.x - 1.0).abs() < 1e-6);
                assert!((p.y - 1.0).abs() < 1e-6);
            },
            _ => panic!("Should intersect at (1, 1)"),
        }
    }

    #[test]
    fn test_parallel_lines() {
        let l1 = [0.0f64, 0.0, 1.0, 1.0];
        let l2 = [0.0f64, 1.0, 1.0, 2.0];
        let n = vec_to_line_segment2!(l1);
        let m = vec_to_line_segment2!(l2);
        match line_intersection_2d(n, m) {
            LineIntersection::Parallel => {},
            _ => panic!("Should be parallel"),
        }
    }

    #[test]
    fn test_coincident_lines() {
        let l1 = [0.0f32, 0.0, 1.0, 1.0];
        let l2 = [0.5f32, 0.5, 1.5, 1.5];
        let n = vec_to_line_segment2!(l1);
        let m = vec_to_line_segment2!(l2);
        match line_intersection_2d(n, m) {
            LineIntersection::Coincident => {},
            _ => panic!("Should be coincident"),
        }
    }

    #[test]
    fn test_almost_parallel_lines() {
        let l1 = [0.0f64, 0.0, 1.0, 1.0];
        let l2 = [0.0f64, 1.0, 1.0, 1.999999];
        let n = vec_to_line_segment2!(l1);
        let m = vec_to_line_segment2!(l2);
        let expect = [1000000.0000823, 1000000.0000823];
        match line_intersection_2d(n, m) {
            LineIntersection::Point(p) => {
                assert!((p.x - expect[0]).abs() < 1e-6);
                assert!((p.y - expect[1]).abs() < 1e-6);
            },
            _ => panic!("Should intersect"),
        }
    }
}
