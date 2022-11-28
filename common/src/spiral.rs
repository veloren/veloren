use vek::*;

/// An iterator of coordinates that create a rectangular spiral out from the
/// origin
#[derive(Clone)]
pub struct Spiral2d {
    layer: i32,
    i: i32,
}

impl Spiral2d {
    #[allow(clippy::new_without_default)]
    /// Creates a new spiral starting at the origin
    pub fn new() -> Self { Self { layer: 0, i: 0 } }

    /// Creates an iterator over points in a spiral starting at the origin and
    /// going out to some radius
    pub fn with_radius(radius: i32) -> impl Iterator<Item = Vec2<i32>> {
        Self::new()
            .take((radius * 2 + 1).pow(2) as usize)
            .filter(move |pos| pos.magnitude_squared() < (radius + 1).pow(2))
    }

    /// Creates an iterator over points in the edge of a circle of some radius
    pub fn with_edge_radius(radius: i32) -> impl Iterator<Item = Vec2<i32>> {
        Self::new()
            .take((radius * 2 + 1).pow(2) as usize)
            .filter(move |pos| pos.magnitude_squared() < (radius + 1).pow(2))
            .filter(move |pos| pos.magnitude_squared() >= radius.pow(2))
    }
}

impl Iterator for Spiral2d {
    type Item = Vec2<i32>;

    #[allow(clippy::erasing_op, clippy::identity_op)]
    fn next(&mut self) -> Option<Self::Item> {
        let layer_size = (self.layer * 8 + 4 * self.layer.min(1) - 4).max(1);
        if self.i >= layer_size {
            self.layer += 1;
            self.i = 0;
        }
        let layer_size = (self.layer * 8 + 4 * self.layer.min(1) - 4).max(1);

        let pos = Vec2::new(
            -self.layer + (self.i - (layer_size / 4) * 0).clamp(0, self.layer * 2)
                - (self.i - (layer_size / 4) * 2).clamp(0, self.layer * 2),
            -self.layer + (self.i - (layer_size / 4) * 1).clamp(0, self.layer * 2)
                - (self.i - (layer_size / 4) * 3).clamp(0, self.layer * 2),
        );

        self.i += 1;

        Some(pos)
    }
}
