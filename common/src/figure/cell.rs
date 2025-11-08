use crate::vol::FilledVox;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, num_derive::FromPrimitive)]
#[repr(u8)]
pub enum CellSurface {
    Matte = 0,
    Glowy = 1,
    Shiny = 2,
    Fire = 3,
    Water = 4,
    MagicCrystal = 5,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "CellSurface")]
// Bits
// 0..5 = CellSurface
// 5..7 = Fill (0 = empty, 1 = hollow, 2 = filled, 3 = override hollow)
pub struct CellAttr(u8);

impl From<CellSurface> for CellAttr {
    fn from(surf: CellSurface) -> Self { Self::filled(surf, false) }
}

impl CellAttr {
    const EMPTY: u8 = 0 << 5;
    const FILLED: u8 = 2 << 5;
    const FILL_MASK: u8 = 0b11 << 5;
    const HOLLOW: u8 = 1 << 5;
    const OVERRIDE: u8 = 3 << 5;

    #[inline]
    fn empty(is_hollow: bool) -> Self { Self(if is_hollow { Self::HOLLOW } else { Self::EMPTY }) }

    #[inline]
    fn filled(surf: CellSurface, is_override: bool) -> Self {
        Self(
            surf as u8
                | if is_override {
                    Self::OVERRIDE
                } else {
                    Self::FILLED
                },
        )
    }

    #[inline]
    pub fn get_surf(&self) -> Option<CellSurface> {
        if self.is_filled() {
            CellSurface::from_u8(self.0 & 0b11111)
        } else {
            None
        }
    }

    #[inline]
    pub fn is_filled(&self) -> bool {
        matches!(self.0 & Self::FILL_MASK, Self::FILLED | Self::OVERRIDE)
    }

    #[inline]
    pub fn is_override_hollow(&self) -> bool { self.0 & Self::FILL_MASK == Self::OVERRIDE }

    #[inline]
    pub fn is_hollowing(&self) -> bool { self.0 & Self::FILL_MASK == Self::HOLLOW }
}

/// A type representing a single voxel in a figure.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    attr: CellAttr,
    #[serde(default)]
    col: Rgb<u8>,
}

const _: () = assert!(4 == std::mem::size_of::<Cell>());
const _: () = assert!(1 == std::mem::align_of::<Cell>());

impl Cell {
    #[inline]
    pub fn empty() -> Self {
        Self {
            col: Rgb::zero(),
            attr: CellAttr::empty(false),
        }
    }

    #[inline]
    pub fn filled(col: Rgb<u8>, surf: CellSurface) -> Self {
        Self {
            col,
            attr: CellAttr::filled(surf, false),
        }
    }

    #[inline]
    pub fn from_index(index: u8, col: Rgb<u8>) -> Cell {
        match index {
            8..13 => Self::filled(col, CellSurface::Shiny),
            13..16 => Self::filled(col, CellSurface::Glowy),
            16 => Self {
                col,
                attr: CellAttr::empty(true),
            },
            17..22 => Self {
                col,
                attr: CellAttr::filled(CellSurface::Matte, true),
            },
            _ => Self::filled(col, CellSurface::Matte),
        }
    }

    #[inline]
    pub fn get_color(&self) -> Option<Rgb<u8>> {
        if self.is_filled() {
            Some(self.col)
        } else {
            None
        }
    }

    /// Transform cell colors
    #[must_use]
    pub fn map_rgb(mut self, transform: impl Fn(Rgb<u8>) -> Rgb<u8>) -> Self {
        self.col = transform(self.col);
        self
    }
}

impl core::ops::Deref for Cell {
    type Target = CellAttr;

    fn deref(&self) -> &Self::Target { &self.attr }
}

impl FilledVox for Cell {
    #[inline]
    fn default_non_filled() -> Self { Cell::empty() }

    #[inline]
    fn is_filled(&self) -> bool { self.attr.is_filled() }
}
