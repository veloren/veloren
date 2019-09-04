use crate::util::Grid;
use common::vol::{BaseVol, ReadVol, Vox, WriteVol};
use rand::prelude::*;
use std::ops::Range;
use vek::*;

#[derive(Clone)]
pub enum ColumnKind {
    Road,
    //Wall,
    Internal,
    //External, // Outside the boundary wall
}

#[derive(Clone, Default)]
pub struct TownColumn {
    pub ground: i32,
    pub kind: Option<ColumnKind>,
}

impl TownColumn {
    pub fn is_empty(&self) -> bool {
        self.kind.is_none()
    }

    pub fn is_road(&self) -> bool {
        self.kind
            .as_ref()
            .map(|kind| match kind {
                ColumnKind::Road => true,
                _ => false,
            })
            .unwrap_or(false)
    }
}

#[derive(Clone)]
pub struct Module {
    pub vol_idx: usize,
    pub dir: usize,
}

#[derive(Clone)]
pub enum CellKind {
    Empty,
    Park,
    Rock,
    Road,
    Wall,
    House(usize),
}

#[derive(Clone)]
pub struct TownCell {
    pub kind: CellKind,
    pub module: Option<Module>,
}

impl TownCell {
    pub fn is_road(&self) -> bool {
        match self.kind {
            CellKind::Road => true,
            _ => false,
        }
    }

    pub fn is_space(&self) -> bool {
        match self.kind {
            CellKind::Empty => true,
            CellKind::Park => true,
            CellKind::Road => true,
            _ => false,
        }
    }

    pub fn is_foundation(&self) -> bool {
        match self.kind {
            CellKind::Rock => true,
            _ => false,
        }
    }
}

impl Vox for TownCell {
    fn empty() -> Self {
        Self {
            kind: CellKind::Empty,
            module: None,
        }
    }

    fn is_empty(&self) -> bool {
        match self.kind {
            CellKind::Empty => true,
            _ => false,
        }
    }
}

impl From<CellKind> for TownCell {
    fn from(kind: CellKind) -> Self {
        Self { kind, module: None }
    }
}

#[derive(Debug)]
pub enum TownError {
    OutOfBounds,
}

const HEIGHT: usize = 24;
const UNDERGROUND_DEPTH: i32 = 5;

type GridItem = (i32, TownColumn, Vec<TownCell>);

pub struct TownVol {
    grid: Grid<GridItem>,
}

impl TownVol {
    pub fn generate_from(
        size: Vec2<i32>,
        mut f: impl FnMut(Vec2<i32>) -> (i32, TownColumn),
        mut g: impl FnMut((&TownColumn, Vec3<i32>)) -> TownCell,
    ) -> Self {
        let mut this = Self {
            grid: Grid::new(
                (0, TownColumn::default(), vec![TownCell::empty(); HEIGHT]),
                size,
            ),
        };

        for (pos, (base, col, cells)) in this.grid.iter_mut() {
            let column = f(pos);
            *base = column.0;
            *col = column.1;
            for z in 0..HEIGHT {
                cells[z] = g((
                    col,
                    Vec3::new(pos.x, pos.y, *base - UNDERGROUND_DEPTH + z as i32),
                ));
            }
        }

        this
    }

    pub fn size(&self) -> Vec2<i32> {
        self.grid.size()
    }

    pub fn set_col_kind(&mut self, pos: Vec2<i32>, kind: Option<ColumnKind>) {
        self.grid.get_mut(pos).map(|col| col.1.kind = kind);
    }

    pub fn col(&self, pos: Vec2<i32>) -> Option<&TownColumn> {
        self.grid.get(pos).map(|col| &col.1)
    }

    pub fn col_range(&self, pos: Vec2<i32>) -> Option<Range<i32>> {
        self.grid.get(pos).map(|col| {
            let lower = col.0 - UNDERGROUND_DEPTH;
            lower..lower + HEIGHT as i32
        })
    }

    pub fn choose_column(
        &self,
        rng: &mut impl Rng,
        mut f: impl FnMut(Vec2<i32>, &TownColumn) -> bool,
    ) -> Option<Vec2<i32>> {
        self.grid
            .iter()
            .filter(|(pos, col)| f(*pos, &col.1))
            .choose(rng)
            .map(|(pos, _)| pos)
    }

    pub fn choose_cell(
        &self,
        rng: &mut impl Rng,
        mut f: impl FnMut(Vec3<i32>, &TownCell) -> bool,
    ) -> Option<Vec3<i32>> {
        self.grid
            .iter()
            .map(|(pos, (base, _, cells))| {
                cells.iter().enumerate().map(move |(i, cell)| {
                    (
                        Vec3::new(pos.x, pos.y, *base - UNDERGROUND_DEPTH + i as i32),
                        cell,
                    )
                })
            })
            .flatten()
            .filter(|(pos, cell)| f(*pos, *cell))
            .choose(rng)
            .map(|(pos, _)| pos)
    }
}

impl BaseVol for TownVol {
    type Vox = TownCell;
    type Err = TownError;
}

impl ReadVol for TownVol {
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, Self::Err> {
        match self.grid.get(Vec2::from(pos)) {
            Some((base, _, cells)) => cells
                .get((pos.z + UNDERGROUND_DEPTH - *base) as usize)
                .ok_or(TownError::OutOfBounds),
            None => Err(TownError::OutOfBounds),
        }
    }
}

impl WriteVol for TownVol {
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), Self::Err> {
        match self.grid.get_mut(Vec2::from(pos)) {
            Some((base, _, cells)) => cells
                .get_mut((pos.z + UNDERGROUND_DEPTH - *base) as usize)
                .map(|cell| *cell = vox)
                .ok_or(TownError::OutOfBounds),
            None => Err(TownError::OutOfBounds),
        }
    }
}
