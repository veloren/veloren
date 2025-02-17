pub mod gradient;

use std::ops::{Add, Sub};

use rand::Rng;
use vek::*;

/// A 2d direction.
#[derive(Debug, enum_map::Enum, strum::EnumIter, enumset::EnumSetType)]
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
            2 => Dir::NegX,
            _ => Dir::NegY,
        }
    }

    pub fn from_vec2(vec: Vec2<i32>) -> Dir {
        if vec.x.abs() > vec.y.abs() {
            if vec.x > 0 { Dir::X } else { Dir::NegX }
        } else if vec.y > 0 {
            Dir::Y
        } else {
            Dir::NegY
        }
    }

    pub fn to_dir3(self) -> Dir3 { Dir3::from_dir(self) }

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
    pub fn rotated_cw(self) -> Dir { self.rotated_ccw().opposite() }

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

    /// Create a vec2 where x is in the direction of `self`, and y is anti
    /// clockwise of `self`.
    pub fn vec2(self, x: i32, y: i32) -> Vec2<i32> {
        match self {
            Dir::X => Vec2::new(x, y),
            Dir::NegX => Vec2::new(-x, -y),
            Dir::Y => Vec2::new(y, x),
            Dir::NegY => Vec2::new(-y, -x),
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

    pub fn is_positive(self) -> bool { matches!(self, Dir::X | Dir::Y) }

    pub fn is_negative(self) -> bool { !self.is_positive() }

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

    pub fn split_aabr_offset<T>(self, aabr: Aabr<T>, offset: T) -> [Aabr<T>; 2]
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

    pub fn trim_aabr(self, aabr: Aabr<i32>, amount: i32) -> Aabr<i32> {
        self.extend_aabr(aabr, -amount)
    }

    pub fn extend_aabr(self, aabr: Aabr<i32>, amount: i32) -> Aabr<i32> {
        let offset = self.to_vec2() * amount;
        match self {
            _ if self.is_positive() => Aabr {
                min: aabr.min,
                max: aabr.max + offset,
            },
            _ => Aabr {
                min: aabr.min + offset,
                max: aabr.max,
            },
        }
    }
}

impl std::ops::Neg for Dir {
    type Output = Dir;

    fn neg(self) -> Self::Output { self.opposite() }
}

/// A 3d direction.
#[derive(Debug, enum_map::Enum, strum::EnumIter, enumset::EnumSetType)]
pub enum Dir3 {
    X,
    Y,
    Z,
    NegX,
    NegY,
    NegZ,
}

impl Dir3 {
    pub const ALL: [Dir; 4] = [Dir::X, Dir::Y, Dir::NegX, Dir::NegY];

    pub fn choose(rng: &mut impl Rng) -> Dir3 {
        match rng.gen_range(0..6) {
            0 => Dir3::X,
            1 => Dir3::Y,
            2 => Dir3::Z,
            3 => Dir3::NegX,
            4 => Dir3::NegY,
            _ => Dir3::NegZ,
        }
    }

    pub fn from_dir(dir: Dir) -> Dir3 {
        match dir {
            Dir::X => Dir3::X,
            Dir::Y => Dir3::Y,
            Dir::NegX => Dir3::NegX,
            Dir::NegY => Dir3::NegY,
        }
    }

    pub fn to_dir(self) -> Option<Dir> {
        match self {
            Dir3::X => Some(Dir::X),
            Dir3::Y => Some(Dir::Y),
            Dir3::NegX => Some(Dir::NegX),
            Dir3::NegY => Some(Dir::NegY),
            _ => None,
        }
    }

    pub fn from_vec3(vec: Vec3<i32>) -> Dir3 {
        if vec.x.abs() > vec.y.abs() && vec.x.abs() > vec.z.abs() {
            if vec.x > 0 { Dir3::X } else { Dir3::NegX }
        } else if vec.y.abs() > vec.z.abs() {
            if vec.y > 0 { Dir3::Y } else { Dir3::NegY }
        } else if vec.z > 0 {
            Dir3::Z
        } else {
            Dir3::NegZ
        }
    }

    #[must_use]
    pub fn opposite(self) -> Dir3 {
        match self {
            Dir3::X => Dir3::NegX,
            Dir3::NegX => Dir3::X,
            Dir3::Y => Dir3::NegY,
            Dir3::NegY => Dir3::Y,
            Dir3::Z => Dir3::NegZ,
            Dir3::NegZ => Dir3::Z,
        }
    }

    /// Rotate counter clockwise around an axis by 90 degrees.
    pub fn rotate_axis_ccw(self, axis: Dir3) -> Dir3 {
        match axis {
            Dir3::X | Dir3::NegX => match self {
                Dir3::Y => Dir3::Z,
                Dir3::NegY => Dir3::NegZ,
                Dir3::Z => Dir3::NegY,
                Dir3::NegZ => Dir3::Y,
                x => x,
            },
            Dir3::Y | Dir3::NegY => match self {
                Dir3::X => Dir3::Z,
                Dir3::NegX => Dir3::NegZ,
                Dir3::Z => Dir3::NegX,
                Dir3::NegZ => Dir3::X,
                y => y,
            },
            Dir3::Z | Dir3::NegZ => match self {
                Dir3::X => Dir3::Y,
                Dir3::NegX => Dir3::NegY,
                Dir3::Y => Dir3::NegX,
                Dir3::NegY => Dir3::X,
                z => z,
            },
        }
    }

    /// Rotate clockwise around an axis by 90 degrees.
    pub fn rotate_axis_cw(self, axis: Dir3) -> Dir3 { self.rotate_axis_ccw(axis).opposite() }

    /// Get a direction that is orthogonal to both directions, always a positive
    /// direction.
    pub fn cross(self, other: Dir3) -> Dir3 {
        match (self, other) {
            (Dir3::X | Dir3::NegX, Dir3::Y | Dir3::NegY)
            | (Dir3::Y | Dir3::NegY, Dir3::X | Dir3::NegX) => Dir3::Z,
            (Dir3::X | Dir3::NegX, Dir3::Z | Dir3::NegZ)
            | (Dir3::Z | Dir3::NegZ, Dir3::X | Dir3::NegX) => Dir3::Y,
            (Dir3::Z | Dir3::NegZ, Dir3::Y | Dir3::NegY)
            | (Dir3::Y | Dir3::NegY, Dir3::Z | Dir3::NegZ) => Dir3::X,
            (Dir3::X | Dir3::NegX, Dir3::X | Dir3::NegX) => Dir3::Y,
            (Dir3::Y | Dir3::NegY, Dir3::Y | Dir3::NegY) => Dir3::X,
            (Dir3::Z | Dir3::NegZ, Dir3::Z | Dir3::NegZ) => Dir3::Y,
        }
    }

    #[must_use]
    pub fn abs(self) -> Dir3 {
        match self {
            Dir3::X | Dir3::NegX => Dir3::X,
            Dir3::Y | Dir3::NegY => Dir3::Y,
            Dir3::Z | Dir3::NegZ => Dir3::Z,
        }
    }

    #[must_use]
    pub fn signum(self) -> i32 {
        match self {
            Dir3::X | Dir3::Y | Dir3::Z => 1,
            Dir3::NegX | Dir3::NegY | Dir3::NegZ => -1,
        }
    }

    pub fn to_vec3(self) -> Vec3<i32> {
        match self {
            Dir3::X => Vec3::new(1, 0, 0),
            Dir3::NegX => Vec3::new(-1, 0, 0),
            Dir3::Y => Vec3::new(0, 1, 0),
            Dir3::NegY => Vec3::new(0, -1, 0),
            Dir3::Z => Vec3::new(0, 0, 1),
            Dir3::NegZ => Vec3::new(0, 0, -1),
        }
    }

    /// Is this direction parallel to x
    pub fn is_x(self) -> bool { matches!(self, Dir3::X | Dir3::NegX) }

    /// Is this direction parallel to y
    pub fn is_y(self) -> bool { matches!(self, Dir3::Y | Dir3::NegY) }

    /// Is this direction parallel to z
    pub fn is_z(self) -> bool { matches!(self, Dir3::Z | Dir3::NegZ) }

    pub fn is_positive(self) -> bool { matches!(self, Dir3::X | Dir3::Y | Dir3::Z) }

    pub fn is_negative(self) -> bool { !self.is_positive() }

    /// Returns the component that the direction is parallell to
    pub fn select(self, vec: impl Into<Vec3<i32>>) -> i32 {
        let vec = vec.into();
        match self {
            Dir3::X | Dir3::NegX => vec.x,
            Dir3::Y | Dir3::NegY => vec.y,
            Dir3::Z | Dir3::NegZ => vec.z,
        }
    }

    /// Select one component the direction is parallel to from vec and select
    /// the other components from other
    pub fn select_with(self, vec: impl Into<Vec3<i32>>, other: impl Into<Vec3<i32>>) -> Vec3<i32> {
        let vec = vec.into();
        let other = other.into();
        match self {
            Dir3::X | Dir3::NegX => Vec3::new(vec.x, other.y, other.z),
            Dir3::Y | Dir3::NegY => Vec3::new(other.x, vec.y, other.z),
            Dir3::Z | Dir3::NegZ => Vec3::new(other.x, other.y, vec.z),
        }
    }

    /// Returns the side of an aabb that the direction is pointing to
    pub fn select_aabb<T>(self, aabb: Aabb<T>) -> T {
        match self {
            Dir3::X => aabb.max.x,
            Dir3::NegX => aabb.min.x,
            Dir3::Y => aabb.max.y,
            Dir3::NegY => aabb.min.y,
            Dir3::Z => aabb.max.z,
            Dir3::NegZ => aabb.min.z,
        }
    }

    /// Select one component from the side the direction is pointing to from
    /// aabr and select the other components from other
    pub fn select_aabb_with<T>(self, aabb: Aabb<T>, other: impl Into<Vec3<T>>) -> Vec3<T> {
        let other = other.into();
        match self {
            Dir3::X => Vec3::new(aabb.max.x, other.y, other.z),
            Dir3::NegX => Vec3::new(aabb.min.x, other.y, other.z),
            Dir3::Y => Vec3::new(other.x, aabb.max.y, other.z),
            Dir3::NegY => Vec3::new(other.x, aabb.min.y, other.z),
            Dir3::Z => Vec3::new(other.x, other.y, aabb.max.z),
            Dir3::NegZ => Vec3::new(other.x, other.y, aabb.min.z),
        }
    }

    pub fn split_aabb_offset<T>(self, aabb: Aabb<T>, offset: T) -> [Aabb<T>; 2]
    where
        T: Copy + PartialOrd + Add<T, Output = T> + Sub<T, Output = T>,
    {
        match self {
            Dir3::X => aabb.split_at_x(aabb.min.x + offset),
            Dir3::NegX => {
                let res = aabb.split_at_x(aabb.max.x - offset);
                [res[1], res[0]]
            },
            Dir3::Y => aabb.split_at_y(aabb.min.y + offset),
            Dir3::NegY => {
                let res = aabb.split_at_y(aabb.max.y - offset);
                [res[1], res[0]]
            },
            Dir3::Z => aabb.split_at_z(aabb.min.z + offset),
            Dir3::NegZ => {
                let res = aabb.split_at_z(aabb.max.z - offset);
                [res[1], res[0]]
            },
        }
    }

    pub fn trim_aabb(self, aabb: Aabb<i32>, amount: i32) -> Aabb<i32> {
        self.extend_aabb(aabb, -amount)
    }

    pub fn extend_aabb(self, aabb: Aabb<i32>, amount: i32) -> Aabb<i32> {
        let offset = self.to_vec3() * amount;
        match self {
            _ if self.is_positive() => Aabb {
                min: aabb.min,
                max: aabb.max + offset,
            },
            _ => Aabb {
                min: aabb.min + offset,
                max: aabb.max,
            },
        }
    }
}
impl std::ops::Neg for Dir3 {
    type Output = Dir3;

    fn neg(self) -> Self::Output { self.opposite() }
}
