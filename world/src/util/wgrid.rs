use super::Grid;
use vek::*;

pub struct WGrid<T> {
    cell_size: u32,
    grid: Grid<T>,
}

impl<T> WGrid<T> {
    pub fn new(radius: u32, cell_size: u32, default_cell: T) -> Self
        where T: Clone
    {
        Self {
            cell_size,
            grid: Grid::new(Vec2::broadcast(radius as i32 * 2 + 1), default_cell),
        }
    }

    fn offset(&self) -> Vec2<i32> {
        self.grid.size() / 2
    }

    pub fn get_local(&self, pos: Vec2<i32>) -> Option<&T> {
        self.grid.get(pos + self.offset())
    }
}
