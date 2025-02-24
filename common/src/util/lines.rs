use vek::{LineSegment3, Vec3};

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
