/// Calculate the shortest distance between the surfaces of two shapes
use vek::*;

pub trait FindDist<T> {
    /// Compute roughly whether the other shape is out of range
    /// Meant to be a cheap method for initial filtering
    /// Must return true if the shape could be within the supplied distance but
    /// is allowed to return true if the shape is actually just out of
    /// range
    fn approx_in_range(self, other: T, range: f32) -> bool;
    /// Find the smallest distance between the two shapes
    fn min_distance(self, other: T) -> f32;
}

/// A z-axis aligned cylinder
#[derive(Clone, Copy, Debug)]
pub struct Cylinder {
    /// Center of the cylinder
    pub pos: Vec3<f32>,
    /// Radius of the cylinder
    pub radius: f32,
    /// Height of the cylinder
    pub height: f32,
}

impl Cylinder {
    fn aabb(&self) -> Aabb<f32> {
        Aabb {
            min: self.pos - Vec3::new(self.radius, self.radius, self.height) / 2.0,
            max: self.pos + Vec3::new(self.radius, self.radius, self.height) / 2.0,
        }
    }

    #[inline]
    pub fn from_components(
        pos: Vec3<f32>,
        scale: Option<crate::comp::Scale>,
        collider: Option<crate::comp::Collider>,
        char_state: Option<&crate::comp::CharacterState>,
    ) -> Self {
        let scale = scale.map_or(1.0, |s| s.0);
        let radius = collider.map_or(0.5, |c| c.get_radius()) * scale;
        let z_limit_modifier = char_state
            .filter(|char_state| char_state.is_dodge())
            .map_or(1.0, |_| 0.5)
            * scale;
        let (z_bottom, z_top) = collider
            .map(|c| c.get_z_limits(z_limit_modifier))
            .unwrap_or((-0.5 * z_limit_modifier, 0.5 * z_limit_modifier));

        Self {
            pos: pos + Vec3::unit_z() * (z_top + z_bottom) / 2.0,
            radius,
            height: z_top - z_bottom,
        }
    }
}

/// An axis aligned cube
#[derive(Clone, Copy, Debug)]
pub struct Cube {
    /// The position of min corner of the cube
    pub pos: Vec3<f32>,
    /// The side length of the cube
    pub side_length: f32,
}

impl FindDist<Cylinder> for Cube {
    #[inline]
    fn approx_in_range(self, other: Cylinder, range: f32) -> bool {
        let cube_plus_range_aabb = Aabb {
            min: self.pos - Vec3::from(range),
            max: self.pos + Vec3::from(range),
        };
        let cylinder_aabb = other.aabb();

        cube_plus_range_aabb.collides_with_aabb(cylinder_aabb)
    }

    #[inline]
    fn min_distance(self, other: Cylinder) -> f32 {
        // Distance between centers along the z-axis
        let z_center_dist = (self.pos.z + self.side_length / 2.0 - other.pos.z).abs();
        // Distance between surfaces projected onto the z-axis
        let z_dist = (z_center_dist - (self.side_length + other.height) / 2.0).max(0.0);
        // Distance between shapes projected onto the xy plane as a square/circle
        let square_aabr = Aabr {
            min: self.pos.xy(),
            max: self.pos.xy() + self.side_length,
        };
        let xy_dist = (square_aabr.distance_to_point(other.pos.xy()) - other.radius).max(0.0);
        // Overall distance by pythagoras
        (z_dist.powi(2) + xy_dist.powi(2)).sqrt()
    }
}

impl FindDist<Cube> for Cylinder {
    #[inline]
    fn approx_in_range(self, other: Cube, range: f32) -> bool { other.approx_in_range(self, range) }

    #[inline]
    fn min_distance(self, other: Cube) -> f32 { other.min_distance(self) }
}

impl FindDist<Cylinder> for Cylinder {
    #[inline]
    fn approx_in_range(self, other: Cylinder, range: f32) -> bool {
        let mut aabb = self.aabb();
        aabb.min -= range;
        aabb.max += range;

        aabb.collides_with_aabb(other.aabb())
    }

    #[inline]
    fn min_distance(self, other: Cylinder) -> f32 {
        // Distance between centers along the z-axis
        let z_center_dist = (self.pos.z - other.pos.z).abs();
        // Distance between surfaces projected onto the z-axis
        let z_dist = (z_center_dist - (self.height + other.height) / 2.0).max(0.0);
        // Distance between shapes projected onto the xy plane as a circles
        let xy_dist =
            (self.pos.xy().distance(other.pos.xy()) - self.radius - other.radius).max(0.0);
        // Overall distance by pythagoras
        (z_dist.powi(2) + xy_dist.powi(2)).sqrt()
    }
}

impl FindDist<Vec3<f32>> for Cylinder {
    #[inline]
    fn approx_in_range(self, other: Vec3<f32>, range: f32) -> bool {
        let mut aabb = self.aabb();
        aabb.min -= range;
        aabb.max += range;

        aabb.contains_point(other)
    }

    #[inline]
    fn min_distance(self, other: Vec3<f32>) -> f32 {
        // Distance between center and point along the z-axis
        let z_center_dist = (self.pos.z - other.z).abs();
        // Distance between surface and point projected onto the z-axis
        let z_dist = (z_center_dist - self.height / 2.0).max(0.0);
        // Distance between shapes projected onto the xy plane
        let xy_dist = (self.pos.xy().distance(other.xy()) - self.radius).max(0.0);
        // Overall distance by pythagoras
        (z_dist.powi(2) + xy_dist.powi(2)).sqrt()
    }
}
