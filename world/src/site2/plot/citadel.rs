use super::*;
use crate::{util::NEIGHBORS, Land};
use rand::prelude::*;
use std::ops::{Add, Div, Mul};
use vek::*;

struct Cell {
    alt: i32,
    colonade: Option<i32>,
}

const CELL_SIZE: i32 = 16;

pub struct Citadel {
    name: String,
    _seed: u32,
    origin: Vec3<i32>,
    radius: i32,
    grid: Grid<Option<Cell>>,
}

impl Citadel {
    pub fn generate(wpos: Vec2<i32>, land: &Land, rng: &mut impl Rng) -> Self {
        let alt = land.get_alt_approx(wpos) as i32;

        let name = NameGen::location(rng).generate_town();
        let seed = rng.gen();
        let origin = wpos.with_z(alt);

        let radius = 150;

        let cell_radius = radius / CELL_SIZE;
        let mut grid = Grid::populate_from(Vec2::broadcast((cell_radius + 1) * 2), |pos| {
            let rpos = pos - cell_radius;
            if rpos.magnitude_squared() < cell_radius.pow(2) {
                let height = Lerp::lerp(
                    120.0,
                    24.0,
                    rpos.map(i32::abs).reduce_max() as f32 / cell_radius as f32,
                );
                let level_height = 32.0;
                Some(Cell {
                    alt: land
                        .get_alt_approx(wpos + rpos * CELL_SIZE + CELL_SIZE / 2)
                        .add(height)
                        .div(level_height)
                        .floor()
                        .mul(level_height) as i32,
                    colonade: None,
                })
            } else {
                None
            }
        });

        for y in 0..grid.size().y {
            for x in 0..grid.size().x {
                let pos = Vec2::new(x, y);
                if let Some(min_alt) = NEIGHBORS
                    .into_iter()
                    .filter_map(|rpos| Some(grid.get(pos + rpos)?.as_ref()?.alt))
                    .min()
                {
                    let Some(Some(cell)) = grid.get_mut(pos)
                        else { continue };
                    if min_alt < cell.alt {
                        cell.colonade = Some(min_alt);
                    }
                }
            }
        }

        Self {
            name,
            _seed: seed,
            origin,
            radius,
            grid,
        }
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn radius(&self) -> i32 { self.radius }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            trees: (wpos - self.origin).map(i32::abs).reduce_max() > self.radius,
            waypoints: false,
            ..SpawnRules::default()
        }
    }

    fn wpos_cell(&self, wpos: Vec2<i32>) -> Vec2<i32> {
        (wpos - self.origin) / CELL_SIZE + self.grid.size() / 2
    }

    fn cell_wpos(&self, pos: Vec2<i32>) -> Vec2<i32> {
        (pos - self.grid.size() / 2) * CELL_SIZE + self.origin
    }
}

impl Structure for Citadel {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_citadel\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_citadel")]
    fn render_inner(&self, _site: &Site, land: &Land, painter: &Painter) {
        for (pos, cell) in self.grid.iter_area(
            self.wpos_cell(painter.render_aabr().min) - 1,
            Vec2::<i32>::from(painter.render_aabr().size()) / CELL_SIZE + 2,
        ) {
            if let Some(cell) = cell {
                let wpos = self.cell_wpos(pos);
                // Clear space above
                painter
                    .aabb(Aabb {
                        min: wpos.with_z(cell.alt),
                        max: (wpos + CELL_SIZE).with_z(cell.alt + 16),
                    })
                    .clear();

                let mut prim = painter.aabb(Aabb {
                    min: wpos.with_z(land.get_alt_approx(wpos + CELL_SIZE / 2) as i32 - 32),
                    max: (wpos + CELL_SIZE).with_z(cell.alt),
                });

                // Colonades under cells
                if let Some(colonade_alt) = cell.colonade {
                    let hole = painter
                        .aabb(Aabb {
                            min: wpos.with_z(colonade_alt),
                            max: (wpos + CELL_SIZE).with_z(cell.alt),
                        })
                        .intersect(painter.prim(Primitive::Superquadric {
                            aabb: Aabb {
                                min: (wpos - 1).with_z(colonade_alt - 32),
                                max: (wpos + 1 + CELL_SIZE).with_z(cell.alt - 1),
                            },
                            degree: 2.5,
                        }));
                    hole.clear();
                    prim = prim.without(hole);
                }

                // Walls around cells
                for dir in CARDINALS {
                    if self
                        .grid
                        .get(pos + dir)
                        .and_then(Option::as_ref)
                        .map_or(true, |near| near.alt < cell.alt)
                    {
                        let offset = wpos + CELL_SIZE / 2 + dir * CELL_SIZE / 2;
                        let rad = dir.map(|e| if e == 0 { CELL_SIZE / 2 + 1 } else { 1 });
                        let height = if pos.sum() % 2 == 0 { 5 } else { 2 };
                        prim = prim.union(painter.aabb(Aabb {
                            min: (offset - rad).with_z(cell.alt - 6),
                            max: (offset + rad).with_z(cell.alt + height),
                        }));
                    }
                }

                prim.fill(Fill::Brick(BlockKind::Rock, Rgb::new(100, 100, 100), 20));
            }
        }
    }
}
