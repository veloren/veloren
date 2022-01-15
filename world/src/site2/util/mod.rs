pub mod gradient;

use rand::Rng;
use vek::*;

/// A 2d direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dir {
    X,
    Y,
    NegX,
    NegY,
}

impl Dir {
    pub fn choose(rng: &mut impl Rng) -> Dir {
        match rng.gen_range(0..4) {
            0 => Dir::X,
            1 => Dir::Y,
            3 => Dir::NegX,
            _ => Dir::NegY,
        }
    }

    pub fn from_vector(vec: Vec2<i32>) -> Dir {
        if vec.x.abs() > vec.y.abs() {
            if vec.x > 0 { Dir::X } else { Dir::NegX }
        } else if vec.y > 0 {
            Dir::Y
        } else {
            Dir::NegY
        }
    }

    #[must_use]
    pub fn opposite(self) -> Dir {
        match self {
            Dir::X => Dir::NegX,
            Dir::NegX => Dir::X,
            Dir::Y => Dir::NegY,
            Dir::NegY => Dir::Y,
        }
    }

    /// Rotate the direction anti clock wise
    #[must_use]
    pub fn rotate_left(self) -> Dir {
        match self {
            Dir::X => Dir::Y,
            Dir::NegX => Dir::NegY,
            Dir::Y => Dir::NegX,
            Dir::NegY => Dir::X,
        }
    }

    /// Rotate the direction clock wise
    #[must_use]
    pub fn rotate_right(self) -> Dir {
        match self {
            Dir::X => Dir::NegY,
            Dir::NegX => Dir::Y,
            Dir::Y => Dir::X,
            Dir::NegY => Dir::NegX,
        }
    }

    pub fn to_vec2(self) -> Vec2<i32> {
        match self {
            Dir::X => Vec2::new(1, 0),
            Dir::NegX => Vec2::new(-1, 0),
            Dir::Y => Vec2::new(0, 1),
            Dir::NegY => Vec2::new(0, -1),
        }
    }

    pub fn to_vec3(self) -> Vec3<i32> {
        match self {
            Dir::X => Vec3::new(1, 0, 0),
            Dir::NegX => Vec3::new(-1, 0, 0),
            Dir::Y => Vec3::new(0, 1, 0),
            Dir::NegY => Vec3::new(0, -1, 0),
        }
    }

    /// Returns a 3x3 matrix that rotates Vec3(1, 0, 0) to the direction you get
    /// in to_vec3. Inteded to be used with Primitive::Rotate.
    ///
    /// Example:
    /// ```
    /// use vek::Vec3;
    /// use veloren_world::site2::util::Dir;
    /// let dir = Dir::X;
    ///
    /// assert_eq!(dir.to_mat3x3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir::NegX;
    ///
    /// assert_eq!(dir.to_mat3x3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir::Y;
    ///
    /// assert_eq!(dir.to_mat3x3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir::NegY;
    ///
    /// assert_eq!(dir.to_mat3x3() * Vec3::new(1, 0, 0), dir.to_vec3());
    /// ```
    pub fn to_mat3x3(self) -> Mat3<i32> {
        match self {
            Dir::X => Mat3::new(1, 0, 0, 0, 1, 0, 0, 0, 1),
            Dir::NegX => Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1),
            Dir::Y => Mat3::new(0, -1, 0, 1, 0, 0, 0, 0, 1),
            Dir::NegY => Mat3::new(0, 1, 0, -1, 0, 0, 0, 0, 1),
        }
    }

    /// Translates this direction to worldspace as if it was relative to the
    /// other direction
    #[must_use]
    pub fn relative_to(self, other: Dir) -> Dir {
        match other {
            Dir::X => self,
            Dir::NegX => self.opposite(),
            Dir::Y => self.rotate_right(),
            Dir::NegY => self.rotate_left(),
        }
    }

    /// Is this direction parallel to x
    pub fn is_x(self) -> bool { matches!(self, Dir::X | Dir::NegX) }

    /// Is this direction parallel to y
    pub fn is_y(self) -> bool { matches!(self, Dir::Y | Dir::NegY) }
}

impl std::ops::Neg for Dir {
    type Output = Dir;

    fn neg(self) -> Self::Output { self.opposite() }
}
