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

    /// Creates an iterator over points in the margin between two squares, inclusive of the inner_radius
    /// and exclusive of the outer_radius where outer_radius = inner_radius + margin
    /*
        Spiral2d iterates over the points in a square spiral pattern starting at the bottom left.
        In the ring spiral, the iteration starts at the bottom left of the inner square and
        does not include the outer square (if you think of the outer square as inner_radius + margin).
        +-----------------------+                           
        |        Margin         |                           
        |     +-----------+     |                           
        |     |           |     |                           
        |     |    Not    |     |                           
        |     | Included  |     |                           
        |     |           |     |                           
        |     +-----------+     |                           
        |                       |                           
        +-----------------------+
        For example, Spiral2d::with_ring(1, 2) yields the following output:
            Vec2 { x: -1, y: -1 }
            Vec2 { x: 0, y: -1 }
            Vec2 { x: 1, y: -1 }
            Vec2 { x: 1, y: 0 }
            Vec2 { x: 1, y: 1 }
            Vec2 { x: 0, y: 1 }
            Vec2 { x: -1, y: 1 }
            Vec2 { x: -1, y: 0 }
            Vec2 { x: -2, y: -2 }
            Vec2 { x: -1, y: -2 }
            Vec2 { x: 0, y: -2 }
            Vec2 { x: 1, y: -2 }
            Vec2 { x: 2, y: -2 }
            Vec2 { x: 2, y: -1 }
            Vec2 { x: 2, y: 0 }
            Vec2 { x: 2, y: 1 }
            Vec2 { x: 2, y: 2 }
            Vec2 { x: 1, y: 2 }
            Vec2 { x: 0, y: 2 }
            Vec2 { x: -1, y: 2 }
            Vec2 { x: -2, y: 2 }
            Vec2 { x: -2, y: 1 }
            Vec2 { x: -2, y: 0 }
            Vec2 { x: -2, y: -1 }
        Run the first test below to see this output.
    */
    pub fn with_ring(inner_radius: i32, margin: i32) -> impl Iterator<Item = Vec2<i32>> {
        let outer_radius: i32 = inner_radius + margin - 1;
        let adjusted_inner_radius = if inner_radius > 0 { inner_radius - 1 } else { 0 };
        Spiral2d {
            layer: inner_radius,
            i: 0,
        }.take((outer_radius * 2 + 1).pow(2) as usize - (adjusted_inner_radius * 2 + 1).pow(2) as usize)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_spiral_ring() {
        let spiral = Spiral2d::with_ring(1, 2);
        for pos in spiral {
            println!("{:?}", pos);
        }
    }

    #[test]
    fn empty_spiral_ring() {
        assert_eq!(Spiral2d::with_ring(0, 0).count(), 0);
        assert_eq!(Spiral2d::with_ring(0, 1).count(), 0);
    }

    #[test]
    fn minimum_spiral_ring() {
        let min_spiral_ring: Vec<Vec2<i32>> = vec![
            Vec2::new(-1, -1),
            Vec2::new(0, -1),
            Vec2::new(1, -1),
            Vec2::new(1, 0),
            Vec2::new(1, 1),
            Vec2::new(0, 1),
            Vec2::new(-1, 1),
            Vec2::new(-1, 0),
        ];
        let result: Vec<Vec2<i32>> = Spiral2d::with_ring(1, 1).collect();
        assert_eq!(result, min_spiral_ring);
    }
}
