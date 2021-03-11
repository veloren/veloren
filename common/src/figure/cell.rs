use crate::vol::Vox;
use vek::*;

pub(super) const GLOWY: u8 = 1 << 0;
pub(super) const SHINY: u8 = 1 << 1;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(packed)]
pub struct CellData {
    pub col: Rgb<u8>,
    pub attr: u8, // 0 = glowy, 1 = shiny
}

impl Default for CellData {
    fn default() -> Self {
        Self {
            col: Rgb::broadcast(255),
            attr: 0,
        }
    }
}

/// A type representing a single voxel in a figure.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Filled(CellData),
    Empty,
}

impl Cell {
    pub fn new(col: Rgb<u8>, glowy: bool, shiny: bool) -> Self {
        Cell::Filled(CellData {
            col,
            attr: glowy as u8 * GLOWY + shiny as u8 * SHINY,
        })
    }

    pub fn get_color(&self) -> Option<Rgb<u8>> {
        match self {
            Cell::Filled(data) => Some(data.col),
            Cell::Empty => None,
        }
    }

    pub fn is_glowy(&self) -> bool {
        match self {
            Cell::Filled(data) => data.attr & GLOWY != 0,
            Cell::Empty => false,
        }
    }

    pub fn is_shiny(&self) -> bool {
        match self {
            Cell::Filled(data) => data.attr & SHINY != 0,
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
