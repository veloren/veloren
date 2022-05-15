use crate::{
    util::{
        gen_cache::StructureGenCache, seed_expan, RandomField, Sampler, StructureGen2d,
        UnitChooser, NEIGHBORS, NEIGHBORS3,
    },
    Canvas, ColumnSample, CONFIG,
};
use common::terrain::{Block, BlockKind};
use ordered_float::NotNan;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use vek::*;

struct Rock {
    wpos: Vec3<i32>,
    seed: u32,
    units: Vec2<Vec2<i32>>,
    kind: RockKind,
}

pub fn apply_rocks_to(canvas: &mut Canvas, _dynamic_rng: &mut impl Rng) {
    let mut rock_gen = StructureGenCache::new(StructureGen2d::new(canvas.index().seed, 24, 10));

    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let rocks = rock_gen.get(wpos2d, |wpos, seed| {
            let col = info.col_or_gen(wpos)?;

            let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));

            const BASE_ROCK_DENSITY: f64 = 0.15;
            if rng.gen_bool((BASE_ROCK_DENSITY * col.rock_density as f64).clamped(0.0, 1.0))
                && col.path.map_or(true, |(d, _, _, _)| d > 6.0)
            {
                match (
                    (col.alt - CONFIG.sea_level) as i32,
                    (col.alt - col.water_level) as i32,
                    col.water_dist.map_or(i32::MAX, |d| d as i32),
                ) {
                    (-3..=2, _, _) => {
                        if rng.gen_bool(0.3) {
                            Some(RockKind::Rauk(Pillar::generate(&mut rng)))
                        } else {
                            Some(RockKind::Rock(VoronoiCell::generate(
                                rng.gen_range(1.0..3.0),
                                &mut rng,
                            )))
                        }
                    },
                    (_, -15..=3, _) => Some(RockKind::Rock(VoronoiCell::generate(
                        rng.gen_range(1.0..4.0),
                        &mut rng,
                    ))),
                    (5..=i32::MAX, _, 0..=i32::MAX) => {
                        if col.temp > CONFIG.desert_temp - 0.1
                            && col.humidity < CONFIG.desert_hum + 0.1
                        {
                            Some(RockKind::Sandstone(VoronoiCell::generate(
                                rng.gen_range(2.0..20.0 - 10.0 * col.tree_density),
                                &mut rng,
                            )))
                        } else {
                            Some(RockKind::Rock(VoronoiCell::generate(
                                rng.gen_range(2.0..20.0 - 10.0 * col.tree_density),
                                &mut rng,
                            )))
                        }
                    },
                    _ => None,
                }
                .map(|kind| Rock {
                    wpos: wpos.with_z(col.alt as i32),
                    seed,
                    units: UnitChooser::new(seed).get(seed).into(),
                    kind,
                })
            } else {
                None
            }
        });

        for rock in rocks {
            let bounds = rock.kind.get_bounds();

            let rpos2d = (wpos2d - rock.wpos.xy())
                .map2(rock.units, |p, unit| unit * p)
                .sum();

            if !Aabr::from(bounds).contains_point(rpos2d) {
                // Skip this column
                continue;
            }

            let mut is_top = true;
            let mut last_block = Block::empty();
            for z in (bounds.min.z..bounds.max.z).rev() {
                let wpos = Vec3::new(wpos2d.x, wpos2d.y, rock.wpos.z + z);
                let model_pos = (wpos - rock.wpos)
                    .xy()
                    .map2(rock.units, |rpos, unit| unit * rpos)
                    .sum()
                    .with_z(wpos.z - rock.wpos.z);

                rock.kind
                    .take_sample(model_pos, rock.seed, last_block, col)
                    .map(|block| {
                        if col.snow_cover && is_top && block.is_filled() {
                            canvas.set(
                                wpos + Vec3::unit_z(),
                                Block::new(BlockKind::Snow, Rgb::new(210, 210, 255)),
                            );
                        }
                        canvas.set(wpos, block);
                        is_top = false;
                        last_block = block;
                    });
            }
        }
    });
}

struct VoronoiCell {
    size: f32,
    points: [Vec3<f32>; 26],
}

impl VoronoiCell {
    fn generate(size: f32, rng: &mut impl Rng) -> Self {
        let mut points = [Vec3::zero(); 26];
        for (i, p) in NEIGHBORS3.iter().enumerate() {
            points[i] = p.as_() * size
                + Vec3::new(
                    rng.gen_range(-0.5..=0.5) * size,
                    rng.gen_range(-0.5..=0.5) * size,
                    rng.gen_range(-0.5..=0.5) * size,
                );
        }
        Self { size, points }
    }

    fn sample_at(&self, rpos: Vec3<i32>) -> bool {
        let rposf = rpos.as_();
        // Would theoretically only need to compare with 7 other points rather than 26,
        // by checking all the points in the cells touching the closest corner of this
        // point.
        rposf.magnitude_squared()
            <= *(0..26)
                .map(|i| self.points[i].distance_squared(rposf))
                .map(|d| NotNan::new(d).unwrap())
                .min()
                .unwrap()
    }
}

struct Pillar {
    height: f32,
    max_extent: Vec2<f32>,
    extents: [Vec2<f32>; 3],
}

impl Pillar {
    fn generate(rng: &mut impl Rng) -> Self {
        let extents = [
            Vec2::new(rng.gen_range(0.5..1.5), rng.gen_range(0.5..1.5)),
            Vec2::new(rng.gen_range(0.8..2.8), rng.gen_range(0.8..2.8)),
            Vec2::new(rng.gen_range(0.5..1.5), rng.gen_range(0.5..3.5)),
        ];
        Self {
            height: rng.gen_range(6.0..16.0),
            extents,
            max_extent: extents
                .iter()
                .cloned()
                .reduce(|accum, item| accum.map2(item, |a, b| a.max(b)))
                .unwrap(),
        }
    }

    fn sample_at(&self, rpos: Vec3<i32>) -> bool {
        let h = rpos.z as f32 / self.height;
        let extent = if h < 0.0 {
            self.extents[0] * (-h).max(1.0)
        } else if h < 0.5 {
            self.extents[0].map2(self.extents[1], |l, m| f32::lerp(l, m, h * 2.0))
        } else if h < 1.0 {
            self.extents[1].map2(self.extents[2], |m, t| f32::lerp(m, t, (h - 0.5) * 2.0))
        } else {
            self.extents[2]
        };
        h < 1.0
            && extent
                .map2(rpos.xy(), |e, p| p.abs() < e.ceil() as i32)
                .reduce_and()
    }
}

enum RockKind {
    // A normal rock with a size
    Rock(VoronoiCell),
    Sandstone(VoronoiCell),
    Rauk(Pillar),
    // Arch,
    // Hoodoos,
}

impl RockKind {
    fn take_sample(
        &self,
        rpos: Vec3<i32>,
        seed: u32,
        last_block: Block,
        col: &ColumnSample,
    ) -> Option<Block> {
        // Used to debug get_bounds
        /*
        let bounds = self.get_bounds();
        if rpos
            .map3(
                bounds.min,
                bounds.max,
                |e, a, b| if e == a || e == b { 1 } else { 0 },
            )
            .sum()
            >= 2
        {
            return Some(Block::new(BlockKind::Rock, Rgb::red()));
        }
        */

        match self {
            RockKind::Rock(cell) => {
                if cell.sample_at(rpos) {
                    let mossiness = 0.1
                        + RandomField::new(seed).get_f32(Vec3::zero()) * 0.3
                        + col.humidity * 0.9;
                    Some(
                        if last_block.is_filled()
                            || (rpos.z as f32 / cell.size
                                + RandomField::new(seed).get_f32(rpos) * 0.3
                                > mossiness)
                        {
                            let mut i = 0;
                            Block::new(
                                BlockKind::WeakRock,
                                col.stone_col.map(|c| {
                                    i += 1;
                                    c + RandomField::new(seed).get(rpos) as u8 % 10
                                }),
                            )
                        } else {
                            Block::new(
                                BlockKind::Grass,
                                col.surface_color.map(|e| (e * 255.0) as u8),
                            )
                        },
                    )
                } else {
                    None
                }
            },
            RockKind::Sandstone(cell) => {
                if cell.sample_at(rpos) {
                    let sandiness = 0.3 + RandomField::new(seed).get_f32(Vec3::zero()) * 0.4;
                    Some(
                        if last_block.is_filled()
                            || (rpos.z as f32 / cell.size
                                + RandomField::new(seed).get_f32(rpos) * 0.3
                                > sandiness)
                        {
                            let mut i = 0;
                            Block::new(
                                BlockKind::WeakRock,
                                Rgb::new(220, 160, 100).map(|c| {
                                    i += 1;
                                    c + RandomField::new(seed + i).get(Vec2::zero().with_z(rpos.z))
                                        as u8
                                        % 30
                                }),
                            )
                        } else {
                            Block::new(
                                BlockKind::Grass,
                                col.surface_color.map(|e| (e * 255.0) as u8),
                            )
                        },
                    )
                } else {
                    None
                }
            },
            RockKind::Rauk(pillar) => {
                let max_extent = *pillar
                    .max_extent
                    .map(|e| NotNan::new(e).unwrap())
                    .reduce_max();
                let is_filled = |rpos| {
                    pillar.sample_at(rpos)
                        && RandomField::new(seed).chance(
                            rpos,
                            1.5 - rpos.z as f32 / pillar.height
                                - rpos.xy().as_::<f32>().magnitude() / max_extent,
                        )
                };
                if is_filled(rpos) ||
                // Prevent floating blocks
                (last_block.is_filled()
                    && NEIGHBORS
                        .iter()
                        .map(|n| !is_filled(rpos + n.with_z(0)))
                        .all(|b| b))
                {
                    Some(Block::new(
                        BlockKind::WeakRock,
                        Rgb::new(
                            190 + RandomField::new(seed + 1).get(rpos) as u8 % 10,
                            190 + RandomField::new(seed + 2).get(rpos) as u8 % 10,
                            190 + RandomField::new(seed + 3).get(rpos) as u8 % 10,
                        ),
                    ))
                } else {
                    None
                }
            },
        }
    }

    fn get_bounds(&self) -> Aabb<i32> {
        match self {
            RockKind::Rock(VoronoiCell { size, .. })
            | RockKind::Sandstone(VoronoiCell { size, .. }) => {
                // Need to use full size because rock can bleed over into other cells
                let extent = *size as i32;
                Aabb {
                    min: Vec3::broadcast(-extent),
                    max: Vec3::broadcast(extent),
                }
            },
            RockKind::Rauk(Pillar {
                max_extent: extent,
                height,
                ..
            }) => Aabb {
                min: (-extent.as_()).with_z(-2),
                max: extent.as_().with_z(*height as i32),
            },
        }
    }
}
