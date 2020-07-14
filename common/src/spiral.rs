use vek::*;

/// An iterator of coordinates that create a rectangular spiral out from the
/// origin
#[derive(Clone)]
pub struct Spiral2d {
    layer: i32,
    i: i32,
}

impl Spiral2d {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self { Self { layer: 0, i: 0 } }
}

impl Iterator for Spiral2d {
    type Item = Vec2<i32>;

    #[allow(clippy::erasing_op)]
    fn next(&mut self) -> Option<Self::Item> {
        let layer_size = (self.layer * 8 + 4 * self.layer.min(1) - 4).max(1);
        if self.i >= layer_size {
            self.layer += 1;
            self.i = 0;
        }
        let layer_size = (self.layer * 8 + 4 * self.layer.min(1) - 4).max(1);

        let pos = Vec2::new(
            -self.layer + (self.i - (layer_size / 4) * 0).max(0).min(self.layer * 2)
                - (self.i - (layer_size / 4) * 2).max(0).min(self.layer * 2),
            -self.layer + (self.i - (layer_size / 4) * 1).max(0).min(self.layer * 2)
                - (self.i - (layer_size / 4) * 3).max(0).min(self.layer * 2),
        );

        self.i += 1;

        Some(pos)
    }
}
