use std::num::NonZeroU8;

use crate::vol::FilledVox;
use vek::*;

const GLOWY: u8 = 1 << 1;
const SHINY: u8 = 1 << 2;
const HOLLOW: u8 = 1 << 3;
const NOT_OVERRIDABLE: u8 = 1 << 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
// 1 = glowy, 2 = shiny, 3 = hollow, 4 = not overridable
pub struct CellAttr(NonZeroU8);

impl CellAttr {
    pub fn new(glowy: bool, shiny: bool, hollow: bool, ignore_hollow: bool) -> Self {
        Self(
            NonZeroU8::new(
                1 + glowy as u8 * GLOWY
                    + shiny as u8 * SHINY
                    + hollow as u8 * HOLLOW
                    + ignore_hollow as u8 * NOT_OVERRIDABLE,
            )
            .expect("At least 1"),
        )
    }

    pub fn from_index(index: u8) -> CellAttr {
        Self::new(
            (13..16).contains(&index), // Glow
            (8..13).contains(&index),  // Shiny
            index == 16,               // Hollow
            (17..22).contains(&index), // Not overridable
        )
    }

    pub fn empty() -> Self { Self(NonZeroU8::new(1).expect("Not zero")) }

    pub fn is_glowy(&self) -> bool { self.0.get() & GLOWY != 0 }

    pub fn is_shiny(&self) -> bool { self.0.get() & SHINY != 0 }

    pub fn is_hollow(&self) -> bool { self.0.get() & HOLLOW != 0 }

    pub fn is_not_overridable(&self) -> bool { self.0.get() & NOT_OVERRIDABLE != 0 }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CellData {
    pub col: Rgb<u8>,
    pub attr: CellAttr,
}

impl CellData {
    pub(super) fn new(col: Rgb<u8>, attr: CellAttr) -> Self { CellData { col, attr } }
}

impl Default for CellData {
    fn default() -> Self { Self::new(Rgb::broadcast(255), CellAttr::empty()) }
}

/// A type representing a single voxel in a figure.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Filled(CellData),
    Empty,
}

impl Cell {
    pub fn new(col: Rgb<u8>, attr: CellAttr) -> Self { Cell::Filled(CellData::new(col, attr)) }

    pub fn get_color(&self) -> Option<Rgb<u8>> {
        match self {
            Cell::Filled(data) => Some(data.col),
            Cell::Empty => None,
        }
    }

    pub fn attr(&self) -> CellAttr {
        match self {
            Cell::Filled(data) => data.attr,
            Cell::Empty => CellAttr::empty(),
        }
    }
}

impl FilledVox for Cell {
    fn default_non_filled() -> Self { Cell::Empty }

    fn is_filled(&self) -> bool { matches!(self, Cell::Filled(_)) }
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
