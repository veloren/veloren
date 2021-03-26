use std::num::NonZeroU8;

use crate::vol::Vox;
use vek::*;

const GLOWY: u8 = 1 << 1;
const SHINY: u8 = 1 << 2;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellData {
    pub col: Rgb<u8>,
    pub attr: NonZeroU8, // 1 = glowy, 2 = shiny
}

impl CellData {
    pub(super) fn new(col: Rgb<u8>, glowy: bool, shiny: bool) -> Self {
        CellData {
            col,
            attr: NonZeroU8::new(1 + glowy as u8 * GLOWY + shiny as u8 * SHINY).unwrap(),
        }
    }
}

impl Default for CellData {
    fn default() -> Self { Self::new(Rgb::broadcast(255), false, false) }
}

/// A type representing a single voxel in a figure.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Filled(CellData),
    Empty,
}

impl Cell {
    pub fn new(col: Rgb<u8>, glowy: bool, shiny: bool) -> Self {
        Cell::Filled(CellData::new(col, glowy, shiny))
    }

    pub fn get_color(&self) -> Option<Rgb<u8>> {
        match self {
            Cell::Filled(data) => Some(data.col),
            Cell::Empty => None,
        }
    }

    pub fn is_glowy(&self) -> bool {
        match self {
            Cell::Filled(data) => data.attr.get() & GLOWY != 0,
            Cell::Empty => false,
        }
    }

    pub fn is_shiny(&self) -> bool {
        match self {
            Cell::Filled(data) => data.attr.get() & SHINY != 0,
            Cell::Empty => false,
        }
    }
}

impl Vox for Cell {
    fn empty() -> Self { Cell::Empty }

    fn is_empty(&self) -> bool {
        match self {
            Cell::Filled(_) => false,
            Cell::Empty => true,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn cell_size() {
        assert_eq!(4, std::mem::size_of::<Cell>());
        assert_eq!(1, std::mem::align_of::<Cell>());
    }
}
