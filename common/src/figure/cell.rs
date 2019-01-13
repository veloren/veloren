// Library
use vek::*;

/// A type representing a single voxel in a figure
#[derive(Copy, Clone, Debug)]
pub enum Cell {
    Filled([u8; 3]),
    Empty,
}

impl Cell {
    pub fn empty() -> Self {
        Cell::Empty
    }

    pub fn new(rgb: Rgb<u8>) -> Self {
        Cell::Filled(rgb.into_array())
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Cell::Filled(_) => false,
            Cell::Empty => true,
        }
    }

    pub fn get_color(&self) -> Option<Rgb<u8>> {
        match self {
            Cell::Filled(col) => Some(Rgb::from(*col)),
            Cell::Empty => None,
        }
    }
}
