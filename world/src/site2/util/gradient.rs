use vek::*;

/// A wrapping mode, used to determine what to do when sampling outside of 0..=1
#[derive(Clone, Copy)]
pub enum WrapMode {
    /// ..............______
    /// No repeat ___/
    Clamp,
    /// Saw wave repeat / / / /
    Repeat,
    /// Triangle wave repeat /\/\/\/\/
    PingPong,
}

impl WrapMode {
    fn sample(&self, t: f32) -> f32 {
        match self {
            WrapMode::Clamp => t.clamp(0.0, 1.0),
            WrapMode::Repeat => (1.0 + t.fract()).fract(),
            WrapMode::PingPong => 1.0 - 2.0 * ((t / 2.0).fract().abs() - 0.5).abs(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Shape {
    Point,
    /// Vector should be normalized for Gradient size to work properly
    Plane(Vec3<f32>),
    /// Vector should be normalized for Gradient size to work properly
    Line(Vec3<f32>),
}

impl Shape {
    /// Create a new plane shape with the given normal.
    pub fn plane(normal: Vec3<f32>) -> Self { Shape::Plane(normal.normalized()) }

    /// Create an infinite line shape with the given direction.
    pub fn radial_line(direction: Vec3<f32>) -> Self { Shape::Line(direction.normalized()) }
}

#[derive(Clone)]
pub struct Gradient {
    /// The center of the gradient shape
    pub(super) center: Vec3<f32>,
    /// The distance the gradient is sampled along
    pub(super) size: f32,
    /// The shape that the distance is computed to to get the gradient color.
    pub(super) shape: Shape,
    /// How the graduint should repeat when the distance from the shape is
    /// greater than size
    pub(super) repeat: WrapMode,
    /// The colors the gradient is lerped between
    pub(super) colors: (Rgb<u8>, Rgb<u8>),
}

impl Gradient {
    pub fn new(center: Vec3<f32>, size: f32, shape: Shape, colors: (Rgb<u8>, Rgb<u8>)) -> Self {
        Gradient {
            center,
            size,
            shape,
            repeat: WrapMode::Clamp,
            colors,
        }
    }

    /// Add a repeat mode to the gradient
    #[must_use]
    pub fn with_repeat(mut self, repeat: WrapMode) -> Self {
        self.repeat = repeat;
        self
    }

    /// Sample the gradient at a certain point, will always return a color
    /// that's in the range color.0..=color.1
    pub fn sample(&self, pos: Vec3<f32>) -> Rgb<u8> {
        // Calculate t by dividing the distance from the shape divided by size
        let t = self.repeat.sample(match self.shape {
            Shape::Point => pos.distance(self.center) / self.size,
            Shape::Plane(normal) => (pos - self.center).dot(normal) / self.size,
            Shape::Line(line) => {
                let u = pos - self.center;
                (u.dot(line) * line - u).magnitude() / self.size
            },
        });
        // Lerp colors
        self.colors.0.map2(self.colors.1, |a, b| {
            (a as f32 * (1.0 - t) + b as f32 * t) as u8
        })
    }
}
