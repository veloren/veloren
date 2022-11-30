pub mod gradient;

use std::ops::{Add, Sub};

use rand::Rng;
use vek::*;

/// A 2d direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    X,
    Y,
    NegX,
    NegY,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::X, Dir::Y, Dir::NegX, Dir::NegY];

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
    pub fn rotated_ccw(self) -> Dir {
        match self {
            Dir::X => Dir::Y,
            Dir::NegX => Dir::NegY,
            Dir::Y => Dir::NegX,
            Dir::NegY => Dir::X,
        }
    }

    /// Rotate the direction clock wise
    #[must_use]
    pub fn rotated_cw(self) -> Dir {
        match self {
            Dir::X => Dir::NegY,
            Dir::NegX => Dir::Y,
            Dir::Y => Dir::X,
            Dir::NegY => Dir::NegX,
        }
    }

    #[must_use]
    pub fn orthogonal(self) -> Dir {
        match self {
            Dir::X | Dir::NegX => Dir::Y,
            Dir::Y | Dir::NegY => Dir::X,
        }
    }

    #[must_use]
    pub fn abs(self) -> Dir {
        match self {
            Dir::X | Dir::NegX => Dir::X,
            Dir::Y | Dir::NegY => Dir::Y,
        }
    }

    #[must_use]
    pub fn signum(self) -> i32 {
        match self {
            Dir::X | Dir::Y => 1,
            Dir::NegX | Dir::NegY => -1,
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
    /// assert_eq!(dir.to_mat3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir::NegX;
    ///
    /// assert_eq!(dir.to_mat3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir::Y;
    ///
    /// assert_eq!(dir.to_mat3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir::NegY;
    ///
    /// assert_eq!(dir.to_mat3() * Vec3::new(1, 0, 0), dir.to_vec3());
    /// ```
    pub fn to_mat3(self) -> Mat3<i32> {
        match self {
            Dir::X => Mat3::new(1, 0, 0, 0, 1, 0, 0, 0, 1),
            Dir::NegX => Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1),
            Dir::Y => Mat3::new(0, -1, 0, 1, 0, 0, 0, 0, 1),
            Dir::NegY => Mat3::new(0, 1, 0, -1, 0, 0, 0, 0, 1),
        }
    }

    /// Creates a matrix that tranforms an upwards facing vector to this
    /// direction.
    pub fn from_z_mat3(self) -> Mat3<i32> {
        match self {
            Dir::X => Mat3::new(0, 0, -1, 0, 1, 0, 1, 0, 0),
            Dir::NegX => Mat3::new(0, 0, 1, 0, 1, 0, -1, 0, 0),
            Dir::Y => Mat3::new(1, 0, 0, 0, 0, -1, 0, 1, 0),
            Dir::NegY => Mat3::new(1, 0, 0, 0, 0, 1, 0, -1, 0),
        }
    }

    /// Translates this direction to worldspace as if it was relative to the
    /// other direction
    #[must_use]
    pub fn relative_to(self, other: Dir) -> Dir {
        match other {
            Dir::X => self,
            Dir::NegX => self.opposite(),
            Dir::Y => self.rotated_cw(),
            Dir::NegY => self.rotated_ccw(),
        }
    }

    /// Is this direction parallel to x
    pub fn is_x(self) -> bool { matches!(self, Dir::X | Dir::NegX) }

    /// Is this direction parallel to y
    pub fn is_y(self) -> bool { matches!(self, Dir::Y | Dir::NegY) }

    /// Returns the component that the direction is parallell to
    pub fn select(self, vec: impl Into<Vec2<i32>>) -> i32 {
        let vec = vec.into();
        match self {
            Dir::X | Dir::NegX => vec.x,
            Dir::Y | Dir::NegY => vec.y,
        }
    }

    /// Select one component the direction is parallel to from vec and select
    /// the other component from other
    pub fn select_with(self, vec: impl Into<Vec2<i32>>, other: impl Into<Vec2<i32>>) -> Vec2<i32> {
        let vec = vec.into();
        let other = other.into();
        match self {
            Dir::X | Dir::NegX => Vec2::new(vec.x, other.y),
            Dir::Y | Dir::NegY => Vec2::new(other.x, vec.y),
        }
    }

    /// Returns the side of an aabr that the direction is pointing to
    pub fn select_aabr<T>(self, aabr: Aabr<T>) -> T {
        match self {
            Dir::X => aabr.max.x,
            Dir::NegX => aabr.min.x,
            Dir::Y => aabr.max.y,
            Dir::NegY => aabr.min.y,
        }
    }

    /// Select one component from the side the direction is pointing to from
    /// aabr and select the other component from other
    pub fn select_aabr_with<T>(self, aabr: Aabr<T>, other: impl Into<Vec2<T>>) -> Vec2<T> {
        let other = other.into();
        match self {
            Dir::X => Vec2::new(aabr.max.x, other.y),
            Dir::NegX => Vec2::new(aabr.min.x, other.y),
            Dir::Y => Vec2::new(other.x, aabr.max.y),
            Dir::NegY => Vec2::new(other.x, aabr.min.y),
        }
    }

    /// The equivelant sprite direction of the direction
    pub fn sprite_ori(self) -> u8 {
        match self {
            Dir::X => 2,
            Dir::NegX => 6,
            Dir::Y => 4,
            Dir::NegY => 0,
        }
    }

    pub fn split_aabr<T>(self, aabr: Aabr<T>, offset: T) -> [Aabr<T>; 2]
    where
        T: Copy + PartialOrd + Add<T, Output = T> + Sub<T, Output = T>,
    {
        match self {
            Dir::X => aabr.split_at_x(aabr.min.x + offset),
            Dir::Y => aabr.split_at_y(aabr.min.y + offset),
            Dir::NegX => {
                let res = aabr.split_at_x(aabr.max.x - offset);
                [res[1], res[0]]
            },
            Dir::NegY => {
                let res = aabr.split_at_y(aabr.max.y - offset);
                [res[1], res[0]]
            },
        }
    }

    pub fn trim_aabr(self, aabr: Aabr<i32>, offset: i32) -> Aabr<i32> {
        Aabr {
            min: aabr.min + self.abs().to_vec2() * offset,
            max: aabr.max - self.abs().to_vec2() * offset,
        }
    }

    pub fn extend_aabr(self, aabr: Aabr<i32>, amount: i32) -> Aabr<i32> {
        match self {
            Dir::X => Aabr {
                min: aabr.min,
                max: aabr.max + Vec2::new(amount, 0),
            },
            Dir::Y => Aabr {
                min: aabr.min,
                max: aabr.max + Vec2::new(0, amount),
            },
            Dir::NegX => Aabr {
                min: aabr.min - Vec2::new(amount, 0),
                max: aabr.max,
            },
            Dir::NegY => Aabr {
                min: aabr.min - Vec2::new(0, amount),
                max: aabr.max,
            },
        }
    }
}

impl std::ops::Neg for Dir {
    type Output = Dir;

    fn neg(self) -> Self::Output { self.opposite() }
}
