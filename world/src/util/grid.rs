use vek::*;

pub struct Grid<T> {
    cells: Vec<T>,
    size: Vec2<i32>,
}

impl<T: Clone> Grid<T> {
    pub fn new(default_cell: T, size: Vec2<i32>) -> Self {
        Self {
            cells: vec![default_cell; size.product() as usize],
            size,
        }
    }

    fn idx(&self, pos: Vec2<i32>) -> usize {
        (pos.y * self.size.x + pos.x) as usize
    }

    pub fn size(&self) -> Vec2<i32> {
        self.size
    }

    pub fn get(&self, pos: Vec2<i32>) -> Option<&T> {
        self.cells.get(self.idx(pos))
    }

    pub fn get_mut(&mut self, pos: Vec2<i32>) -> Option<&mut T> {
        let idx = self.idx(pos);
        self.cells.get_mut(idx)
    }

    pub fn set(&mut self, pos: Vec2<i32>, cell: T) {
        let idx = self.idx(pos);
        self.cells.get_mut(idx).map(|c| *c = cell);
    }

    pub fn iter(&self) -> impl Iterator<Item = (Vec2<i32>, &T)> + '_ {
        (0..self.size.x)
            .map(move |x| {
                (0..self.size.y)
                    .map(move |y| (Vec2::new(x, y), &self.cells[self.idx(Vec2::new(x, y))]))
            })
            .flatten()
    }
}
