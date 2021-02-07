use vek::{Vec2, Vec3};

/// Projection trait for projection of linear types and shapes
pub trait Projection<T> {
    type Output;

    fn projected(self, onto: &T) -> Self::Output;
}

// Impls

impl Projection<Vec2<f32>> for Vec2<f32> {
    type Output = Self;

    fn projected(self, v: &Self) -> Self::Output {
        let v = *v;
        self.dot(v) * v / v.magnitude_squared()
    }
}

impl Projection<Vec3<f32>> for Vec3<f32> {
    type Output = Self;

    fn projected(self, v: &Self) -> Self::Output {
        let v = *v;
        v * self.dot(v) / v.magnitude_squared()
    }
}
