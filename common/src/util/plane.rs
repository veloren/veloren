use super::{Dir, Projection};
use serde::{Deserialize, Serialize};
use vek::*;

/// Plane

// plane defined by its normal and origin
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plane {
    pub normal: Dir,
    /// Distance from origin in the direction of normal
    pub d: f32,
}

impl Plane {
    pub fn new(dir: Dir) -> Self { Self::from(dir) }

    pub fn distance(&self, to: Vec3<f32>) -> f32 { self.normal.dot(to) - self.d }

    // fn center(&self) -> Vec3<f32> { *self.normal * self.d }

    pub fn projection(&self, v: Vec3<f32>) -> Vec3<f32> { v - *self.normal * self.distance(v) }

    pub fn xy() -> Self { Plane::from(Dir::new(Vec3::unit_z())) }

    pub fn yz() -> Self { Plane::from(Dir::new(Vec3::unit_x())) }

    pub fn zx() -> Self { Plane::from(Dir::new(Vec3::unit_y())) }
}

impl From<Dir> for Plane {
    fn from(dir: Dir) -> Self {
        Plane {
            normal: dir,
            d: 0.0,
        }
    }
}

impl Projection<Plane> for Vec3<f32> {
    type Output = Vec3<f32>;

    fn projected(self, plane: &Plane) -> Self::Output { plane.projection(self) }
}

impl<T> Projection<Plane> for Extent2<T>
where
    T: Projection<Plane, Output = T>,
{
    type Output = Self;

    fn projected(self, plane: &Plane) -> Self::Output { self.map(|v| v.projected(plane)) }
}
