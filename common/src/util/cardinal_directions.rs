use std::ops::{Add, Sub};

use rand::RngExt;
use vek::{Aabb, Aabr, Mat3, Vec2, Vec3};

/// A 2d cardinal direction.
#[derive(Debug, enum_map::Enum, strum::EnumIter, enumset::EnumSetType)]
pub enum Dir2 {
    X,
    Y,
    NegX,
    NegY,
}

impl Dir2 {
    pub const ALL: [Dir2; 4] = [Dir2::X, Dir2::Y, Dir2::NegX, Dir2::NegY];

    pub fn choose(rng: &mut impl RngExt) -> Dir2 {
        match rng.random_range(0..4) {
            0 => Dir2::X,
            1 => Dir2::Y,
            2 => Dir2::NegX,
            _ => Dir2::NegY,
        }
    }

    pub fn from_vec2(vec: Vec2<i32>) -> Dir2 {
        if vec.x.abs() > vec.y.abs() {
            if vec.x > 0 { Dir2::X } else { Dir2::NegX }
        } else if vec.y > 0 {
            Dir2::Y
        } else {
            Dir2::NegY
        }
    }

    pub fn to_dir3(self) -> Dir3 { Dir3::from_dir(self) }

    #[must_use]
    pub fn opposite(self) -> Dir2 {
        match self {
            Dir2::X => Dir2::NegX,
            Dir2::NegX => Dir2::X,
            Dir2::Y => Dir2::NegY,
            Dir2::NegY => Dir2::Y,
        }
    }

    /// Rotate the direction anti clock wise
    #[must_use]
    pub fn rotated_ccw(self) -> Dir2 {
        match self {
            Dir2::X => Dir2::Y,
            Dir2::NegX => Dir2::NegY,
            Dir2::Y => Dir2::NegX,
            Dir2::NegY => Dir2::X,
        }
    }

    /// Rotate the direction clock wise
    #[must_use]
    pub fn rotated_cw(self) -> Dir2 { self.rotated_ccw().opposite() }

    #[must_use]
    pub fn orthogonal(self) -> Dir2 {
        match self {
            Dir2::X | Dir2::NegX => Dir2::Y,
            Dir2::Y | Dir2::NegY => Dir2::X,
        }
    }

    #[must_use]
    pub fn abs(self) -> Dir2 {
        match self {
            Dir2::X | Dir2::NegX => Dir2::X,
            Dir2::Y | Dir2::NegY => Dir2::Y,
        }
    }

    #[must_use]
    pub fn signum(self) -> i32 {
        match self {
            Dir2::X | Dir2::Y => 1,
            Dir2::NegX | Dir2::NegY => -1,
        }
    }

    pub fn to_vec2(self) -> Vec2<i32> {
        match self {
            Dir2::X => Vec2::new(1, 0),
            Dir2::NegX => Vec2::new(-1, 0),
            Dir2::Y => Vec2::new(0, 1),
            Dir2::NegY => Vec2::new(0, -1),
        }
    }

    /// The diagonal to the left of `self`, this is equal to this dir plus this
    /// dir rotated counter clockwise.
    pub fn diagonal(self) -> Vec2<i32> { self.to_vec2() + self.rotated_ccw().to_vec2() }

    pub fn to_vec3(self) -> Vec3<i32> {
        match self {
            Dir2::X => Vec3::new(1, 0, 0),
            Dir2::NegX => Vec3::new(-1, 0, 0),
            Dir2::Y => Vec3::new(0, 1, 0),
            Dir2::NegY => Vec3::new(0, -1, 0),
        }
    }

    /// Create a vec2 where x is in the direction of `self`, and y is anti
    /// clockwise of `self`.
    pub fn vec2(self, x: i32, y: i32) -> Vec2<i32> {
        match self {
            Dir2::X => Vec2::new(x, y),
            Dir2::NegX => Vec2::new(-x, -y),
            Dir2::Y => Vec2::new(y, x),
            Dir2::NegY => Vec2::new(-y, -x),
        }
    }

    /// Create a vec2 where x is in the direction of `self`, and y is orthogonal
    /// version of self.
    pub fn vec2_abs<T>(self, x: T, y: T) -> Vec2<T> {
        match self {
            Dir2::X => Vec2::new(x, y),
            Dir2::NegX => Vec2::new(x, y),
            Dir2::Y => Vec2::new(y, x),
            Dir2::NegY => Vec2::new(y, x),
        }
    }

    /// Returns a 3x3 matrix that rotates Vec3(1, 0, 0) to the direction you get
    /// in to_vec3. Inteded to be used with Primitive::Rotate.
    ///
    /// Example:
    /// ```
    /// use vek::Vec3;
    /// use nova_forge_common::util::Dir2;
    /// let dir = Dir2::X;
    ///
    /// assert_eq!(dir.to_mat3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir2::NegX;
    ///
    /// assert_eq!(dir.to_mat3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir2::Y;
    ///
    /// assert_eq!(dir.to_mat3() * Vec3::new(1, 0, 0), dir.to_vec3());
    ///
    /// let dir = Dir2::NegY;
    ///
    /// assert_eq!(dir.to_mat3() * Vec3::new(1, 0, 0), dir.to_vec3());
    /// ```
    pub fn to_mat3(self) -> Mat3<i32> {
        match self {
            Dir2::X => Mat3::new(1, 0, 0, 0, 1, 0, 0, 0, 1),
            Dir2::NegX => Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1),
            Dir2::Y => Mat3::new(0, -1, 0, 1, 0, 0, 0, 0, 1),
            Dir2::NegY => Mat3::new(0, 1, 0, -1, 0, 0, 0, 0, 1),
        }
    }

    /// Creates a matrix that tranforms an upwards facing vector to this
    /// direction.
    pub fn from_z_mat3(self) -> Mat3<i32> {
        match self {
            Dir2::X => Mat3::new(0, 0, -1, 0, 1, 0, 1, 0, 0),
            Dir2::NegX => Mat3::new(0, 0, 1, 0, 1, 0, -1, 0, 0),
            Dir2::Y => Mat3::new(1, 0, 0, 0, 0, -1, 0, 1, 0),
            Dir2::NegY => Mat3::new(1, 0, 0, 0, 0, 1, 0, -1, 0),
        }
    }

    /// Translates this direction to worldspace as if it was relative to the
    /// other direction
    #[must_use]
    pub fn relative_to(self, other: Dir2) -> Dir2 {
        match other {
            Dir2::X => self,
            Dir2::NegX => self.opposite(),
            Dir2::Y => self.rotated_cw(),
            Dir2::NegY => self.rotated_ccw(),
        }
    }

    /// Is this direction parallel to x
    pub fn is_x(self) -> bool { matches!(self, Dir2::X | Dir2::NegX) }

    /// Is this direction parallel to y
    pub fn is_y(self) -> bool { matches!(self, Dir2::Y | Dir2::NegY) }

    pub fn is_positive(self) -> bool { matches!(self, Dir2::X | Dir2::Y) }

    pub fn is_negative(self) -> bool { !self.is_positive() }

    /// Returns the component that the direction is parallell to
    pub fn select(self, vec: impl Into<Vec2<i32>>) -> i32 {
        let vec = vec.into();
        match self {
            Dir2::X | Dir2::NegX => vec.x,
            Dir2::Y | Dir2::NegY => vec.y,
        }
    }

    /// Select one component the direction is parallel to from vec and select
    /// the other component from other
    pub fn select_with(self, vec: impl Into<Vec2<i32>>, other: impl Into<Vec2<i32>>) -> Vec2<i32> {
        let vec = vec.into();
        let other = other.into();
        match self {
            Dir2::X | Dir2::NegX => Vec2::new(vec.x, other.y),
            Dir2::Y | Dir2::NegY => Vec2::new(other.x, vec.y),
        }
    }

    /// Returns the side of an aabr that the direction is pointing to
    pub fn select_aabr<T>(self, aabr: Aabr<T>) -> T {
        match self {
            Dir2::X => aabr.max.x,
            Dir2::NegX => aabr.min.x,
            Dir2::Y => aabr.max.y,
            Dir2::NegY => aabr.min.y,
        }
    }

    /// Select one component from the side the direction is pointing to from
    /// aabr and select the other component from other
    pub fn select_aabr_with<T>(self, aabr: Aabr<T>, other: impl Into<Vec2<T>>) -> Vec2<T> {
        let other = other.into();
        match self {
            Dir2::X => Vec2::new(aabr.max.x, other.y),
            Dir2::NegX => Vec2::new(aabr.min.x, other.y),
            Dir2::Y => Vec2::new(other.x, aabr.max.y),
            Dir2::NegY => Vec2::new(other.x, aabr.min.y),
        }
    }

    /// The equivelant sprite direction of the direction
    pub fn sprite_ori(self) -> u8 {
        match self {
            Dir2::X => 0,
            Dir2::Y => 2,
            Dir2::NegX => 4,
            Dir2::NegY => 6,
        }
    }

    /// Returns (Dir, rest)
    ///
    /// Returns None if `ori` isn't a valid sprite Ori.
    pub fn from_sprite_ori(ori: u8) -> Option<(Dir2, u8)> {
        let dir = match ori / 2 {
            0 => Dir2::X,
            1 => Dir2::Y,
            2 => Dir2::NegX,
            3 => Dir2::NegY,
            _ => return None,
        };
        let rest = ori % 2;

        Some((dir, rest))
    }

    /// Legacy version of `sprite_ori`, so prefer using that over this.
    pub fn sprite_ori_legacy(self) -> u8 {
        match self {
            Dir2::X => 2,
            Dir2::NegX => 6,
            Dir2::Y => 4,
            Dir2::NegY => 0,
        }
    }

    pub fn split_aabr_offset<T>(self, aabr: Aabr<T>, offset: T) -> [Aabr<T>; 2]
    where
        T: Copy + PartialOrd + Add<T, Output = T> + Sub<T, Output = T>,
    {
        match self {
            Dir2::X => aabr.split_at_x(aabr.min.x + offset),
            Dir2::Y => aabr.split_at_y(aabr.min.y + offset),
            Dir2::NegX => {
                let res = aabr.split_at_x(aabr.max.x - offset);
                [res[1], res[0]]
            },
            Dir2::NegY => {
                let res = aabr.split_at_y(aabr.max.y - offset);
                [res[1], res[0]]
            },
        }
    }

    pub fn trim_aabr(self, aabr: Aabr<i32>, amount: i32) -> Aabr<i32> {
        (-self).extend_aabr(aabr, -amount)
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

impl std::ops::Neg for Dir2 {
    type Output = Dir2;

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
    pub const ALL: [Dir2; 4] = [Dir2::X, Dir2::Y, Dir2::NegX, Dir2::NegY];

    pub fn choose(rng: &mut impl RngExt) -> Dir3 {
        match rng.random_range(0..6) {
            0 => Dir3::X,
            1 => Dir3::Y,
            2 => Dir3::Z,
            3 => Dir3::NegX,
            4 => Dir3::NegY,
            _ => Dir3::NegZ,
        }
    }

    pub fn from_dir(dir: Dir2) -> Dir3 {
        match dir {
            Dir2::X => Dir3::X,
            Dir2::Y => Dir3::Y,
            Dir2::NegX => Dir3::NegX,
            Dir2::NegY => Dir3::NegY,
        }
    }

    pub fn to_dir(self) -> Option<Dir2> {
        match self {
            Dir3::X => Some(Dir2::X),
            Dir3::Y => Some(Dir2::Y),
            Dir3::NegX => Some(Dir2::NegX),
            Dir3::NegY => Some(Dir2::NegY),
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
        (-self).extend_aabb(aabb, -amount)
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
