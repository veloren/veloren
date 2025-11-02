use crate::vol::FilledVox;
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellSurface {
    Matte = 0,
    Glowy = 1,
    Shiny = 2,
    Fire = 3,
}

/// A type representing a single voxel in a figure.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    /// `hollowing` determines whether this cell should hollow out others during
    /// masking operations.
    Empty {
        #[serde(default)]
        hollowing: bool,
    },
    Filled {
        #[serde(default)]
        col: Rgb<u8>,
        surf: CellSurface,
        #[serde(default)]
        override_hollow: bool,
    },
}

impl Cell {
    pub fn empty() -> Self { Self::Empty { hollowing: false } }

    pub fn filled(col: Rgb<u8>, surf: CellSurface) -> Self {
        Self::Filled {
            col,
            surf,
            override_hollow: false,
        }
    }

    pub fn from_index(index: u8, col: Rgb<u8>) -> Cell {
        match index {
            8..13 => Self::Filled {
                col,
                surf: CellSurface::Shiny,
                override_hollow: false,
            },
            13..16 => Self::Filled {
                col,
                surf: CellSurface::Glowy,
                override_hollow: false,
            },
            16 => Self::Empty { hollowing: true },
            17..22 => Self::Filled {
                col,
                surf: CellSurface::Matte,
                override_hollow: true,
            },
            _ => Self::Filled {
                col,
                surf: CellSurface::Matte,
                override_hollow: false,
            },
        }
    }

    pub fn get_color(&self) -> Option<Rgb<u8>> {
        match self {
            Cell::Filled { col, .. } => Some(*col),
            Cell::Empty { .. } => None,
        }
    }

    pub fn surf(&self) -> Option<CellSurface> {
        match self {
            Cell::Filled { surf, .. } => Some(*surf),
            Cell::Empty { .. } => None,
        }
    }

    /// Transform cell colors
    #[must_use]
    pub fn map_rgb(self, transform: impl Fn(Rgb<u8>) -> Rgb<u8>) -> Self {
        match self {
            Self::Filled {
                col,
                surf,
                override_hollow,
            } => Self::Filled {
                col: transform(col),
                surf,
                override_hollow,
            },
            this => this,
        }
    }
}

impl FilledVox for Cell {
    fn default_non_filled() -> Self { Cell::empty() }

    fn is_filled(&self) -> bool { matches!(self, Cell::Filled { .. }) }
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
